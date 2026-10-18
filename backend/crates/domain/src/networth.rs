//! Net worth computation in several useful forms, plus snapshots.
//!
//! The engine is pure: it takes accounts, their dated balances, and a set of
//! assumptions, and returns totals with no I/O. See design decisions D4-D6.

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::accounts::{
    Account, AccountClassification, AccountType, Balance, BalanceSource, OwnershipScope,
};
use crate::currency::Currency;
use crate::money::Money;

/// Which net worth figure is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetWorthKind {
    /// Assets minus liabilities.
    Total,
    /// Only liquid assets, minus liabilities.
    Liquid,
    /// Net realizable value after taxes, penalties, and selling costs.
    Accessible,
}

/// Whose accounts to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationScope {
    /// Every account in the household.
    Household,
    /// Shared accounts plus the named user's private accounts.
    Personal,
}

impl AggregationScope {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            AggregationScope::Household => "household",
            AggregationScope::Personal => "personal",
        }
    }
}

/// Errors raised when parsing an [`AggregationScope`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid aggregation scope: {0}")]
pub struct AggregationScopeParseError(pub String);

impl FromStr for AggregationScope {
    type Err = AggregationScopeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "household" => Ok(AggregationScope::Household),
            "personal" => Ok(AggregationScope::Personal),
            other => Err(AggregationScopeParseError(other.to_string())),
        }
    }
}

/// A coarse asset classification, used for dashboard breakdowns.
///
/// Sector-level classification arrives with investment holdings (Group 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    /// Cash and deposit accounts.
    Cash,
    /// Retirement and brokerage accounts.
    Market,
    /// Real property.
    RealEstate,
    /// Anything else.
    Other,
    /// Mortgages and loans.
    Debt,
}

impl AssetClass {
    /// Classify an account type.
    pub fn of(account_type: AccountType) -> Self {
        use AccountType::*;
        match account_type {
            Checking | Savings | Cash => AssetClass::Cash,
            Retirement401k | TraditionalIra | RothIra | TaxableBrokerage => AssetClass::Market,
            RealEstate => AssetClass::RealEstate,
            OtherAsset => AssetClass::Other,
            Mortgage | Loan => AssetClass::Debt,
        }
    }

    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            AssetClass::Cash => "cash",
            AssetClass::Market => "market",
            AssetClass::RealEstate => "real_estate",
            AssetClass::Other => "other",
            AssetClass::Debt => "debt",
        }
    }
}

/// The configurable assumptions used to compute accessible net worth.
///
/// Rates are fractions in `[0, 1]`. v1 uses a single user-set tax rate; a full
/// marginal-bracket engine is a later capability (see design D15/Non-Goals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedemptionPolicy {
    /// Effective tax rate applied to pre-tax retirement assets.
    pub tax_rate: BigDecimal,
    /// Early-withdrawal penalty rate.
    pub early_withdrawal_penalty_rate: BigDecimal,
    /// Age below which the early-withdrawal penalty applies.
    pub early_withdrawal_before_age: u32,
    /// Selling/closing cost rate applied to real estate.
    pub selling_cost_rate: BigDecimal,
    /// Fixed selling/closing costs in the reporting currency.
    pub fixed_costs: BigDecimal,
}

impl Default for RedemptionPolicy {
    fn default() -> Self {
        Self {
            tax_rate: BigDecimal::from(2) / BigDecimal::from(10), // 20%
            early_withdrawal_penalty_rate: BigDecimal::from(10) / BigDecimal::from(100), // 10%
            early_withdrawal_before_age: 59,
            selling_cost_rate: BigDecimal::from(6) / BigDecimal::from(100), // 6%
            fixed_costs: BigDecimal::from(0),
        }
    }
}

/// Inputs to a net worth computation.
#[derive(Debug, Clone)]
pub struct ComputeParams {
    /// The valuation date.
    pub as_of: Date,
    /// Household or personal view.
    pub scope: AggregationScope,
    /// The user whose private accounts are included for a personal view.
    pub owner_user_id: Option<Uuid>,
    /// The owner's age, used for early-withdrawal penalties.
    pub owner_age: Option<u32>,
    /// The reductions applied to accessible net worth.
    pub policy: RedemptionPolicy,
    /// The reporting currency.
    pub currency: Currency,
}

/// One row of a breakdown, in the reporting currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakdownRow {
    /// The group key, such as an account type or asset class.
    pub key: String,
    /// Net amount for the group (liabilities are negative).
    pub amount: Money,
}

/// The computed net worth picture.
#[derive(Debug, Clone)]
pub struct NetWorthBreakdown {
    /// The valuation date.
    pub as_of: Date,
    /// The reporting currency.
    pub currency: Currency,
    /// Assets minus liabilities.
    pub total: Money,
    /// Marketable assets (cash, brokerage, retirement) minus liquid liabilities.
    pub liquid: Money,
    /// Cash and cash equivalents available right now.
    pub liquidity: Money,
    /// Net realizable value.
    pub accessible: Money,
    /// Net amount by account type.
    pub by_account_type: Vec<BreakdownRow>,
    /// Net amount by asset class.
    pub by_asset_class: Vec<BreakdownRow>,
    /// Accounts that contributed a balance.
    pub account_count: usize,
    /// Whether any contributing balance was entered by hand.
    pub has_manual_data: bool,
    /// Whether any contributing balance is older than the freshness threshold.
    pub has_stale_data: bool,
}

/// Balances older than this (in days) are reported as stale.
pub const STALE_AFTER_DAYS: i64 = 45;

/// A persisted, immutable net worth snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Surrogate identifier.
    pub id: Uuid,
    /// The valuation date.
    pub as_of: Date,
    /// Which accounts were included.
    pub scope: AggregationScope,
    /// The user for a personal scope.
    pub owner_user_id: Option<Uuid>,
    /// The age used for penalties.
    pub owner_age: Option<u32>,
    /// The reporting currency.
    pub currency: Currency,
    /// Total net worth at the time.
    pub total: Money,
    /// Liquid net worth at the time.
    pub liquid: Money,
    /// Liquidity (cash on hand) at the time.
    pub liquidity: Money,
    /// Accessible net worth at the time.
    pub accessible: Money,
    /// The policy used at the time (kept for reproducibility).
    pub policy: RedemptionPolicy,
}

/// The values required to persist a snapshot.
#[derive(Debug, Clone)]
pub struct NewSnapshot {
    /// The valuation date.
    pub as_of: Date,
    /// Which accounts were included.
    pub scope: AggregationScope,
    /// The user for a personal scope.
    pub owner_user_id: Option<Uuid>,
    /// The age used for penalties.
    pub owner_age: Option<u32>,
    /// The reporting currency.
    pub currency: Currency,
    /// Total net worth at the time.
    pub total: Money,
    /// Liquid net worth at the time.
    pub liquid: Money,
    /// Liquidity (cash on hand) at the time.
    pub liquidity: Money,
    /// Accessible net worth at the time.
    pub accessible: Money,
    /// The policy used at the time.
    pub policy: RedemptionPolicy,
}

/// Filters for listing snapshots.
#[derive(Debug, Clone, Default)]
pub struct SnapshotFilter {
    /// Restrict to one scope.
    pub scope: Option<AggregationScope>,
    /// Restrict to one owner.
    pub owner_user_id: Option<Uuid>,
}

/// Compute net worth at a point in time from accounts and dated balances.
///
/// Each account contributes at most once: its latest balance on or before
/// `as_of`. Accounts without a balance are skipped.
pub fn compute_net_worth(
    accounts: &[Account],
    balances: &[Balance],
    params: &ComputeParams,
) -> NetWorthBreakdown {
    let mut latest: HashMap<Uuid, &Balance> = HashMap::new();
    for balance in balances {
        if balance.as_of > params.as_of {
            continue;
        }
        match latest.get(&balance.account_id) {
            Some(existing) if existing.as_of >= balance.as_of => {}
            _ => {
                latest.insert(balance.account_id, balance);
            }
        }
    }

    let zero = BigDecimal::from(0);
    let mut total = zero.clone();
    let mut liquid = zero.clone();
    let mut liquidity = zero.clone();
    let mut accessible = zero.clone();
    let mut by_type: BTreeMap<String, BigDecimal> = BTreeMap::new();
    let mut by_class: BTreeMap<String, BigDecimal> = BTreeMap::new();
    let mut account_count = 0usize;
    let mut has_manual_data = false;
    let mut has_stale_data = false;

    for account in accounts {
        if account.archived || !scope_includes(account, params) {
            continue;
        }

        let Some(balance) = latest.get(&account.id) else {
            continue;
        };

        if account.currency != params.currency {
            // FX conversion arrives later; skip rather than mix currencies.
            continue;
        }

        account_count += 1;
        if balance.source == BalanceSource::Manual {
            has_manual_data = true;
        }
        if (params.as_of - balance.as_of).whole_days() > STALE_AFTER_DAYS {
            has_stale_data = true;
        }

        let amount = balance.amount.amount();
        let (signed_total, signed_liquid, signed_liquidity, signed_accessible) =
            match account.classification() {
                AccountClassification::Asset => {
                    let liquid_part = if account.is_liquid() {
                        amount.clone()
                    } else {
                        zero.clone()
                    };
                    let cash_part = if account.account_type.liquidity().is_immediate() {
                        amount.clone()
                    } else {
                        zero.clone()
                    };
                    (
                        amount.clone(),
                        liquid_part,
                        cash_part,
                        accessible_amount(account.account_type, amount, params),
                    )
                }
                AccountClassification::Liability => {
                    let owed = -amount;
                    // Only liquid liabilities reduce liquid net worth, and only
                    // immediate ones reduce liquidity. A mortgage secured by an
                    // excluded property does neither.
                    let liquid_part = if account.account_type.liquidity().is_liquid() {
                        owed.clone()
                    } else {
                        zero.clone()
                    };
                    let cash_part = if account.account_type.liquidity().is_immediate() {
                        owed.clone()
                    } else {
                        zero.clone()
                    };
                    (owed.clone(), liquid_part, cash_part, owed)
                }
            };

        total += &signed_total;
        liquid += &signed_liquid;
        liquidity += &signed_liquidity;
        accessible += &signed_accessible;

        *by_type
            .entry(account.account_type.as_str().to_string())
            .or_insert_with(|| zero.clone()) += &signed_total;
        *by_class
            .entry(AssetClass::of(account.account_type).as_str().to_string())
            .or_insert_with(|| zero.clone()) += &signed_total;
    }

    NetWorthBreakdown {
        as_of: params.as_of,
        currency: params.currency.clone(),
        total: Money::new(total, params.currency.clone()),
        liquid: Money::new(liquid, params.currency.clone()),
        liquidity: Money::new(liquidity, params.currency.clone()),
        accessible: Money::new(accessible, params.currency.clone()),
        by_account_type: to_rows(by_type, &params.currency),
        by_asset_class: to_rows(by_class, &params.currency),
        account_count,
        has_manual_data,
        has_stale_data,
    }
}

fn scope_includes(account: &Account, params: &ComputeParams) -> bool {
    match params.scope {
        AggregationScope::Household => true,
        AggregationScope::Personal => {
            account.ownership_scope == OwnershipScope::Household
                || (params.owner_user_id.is_some() && account.owner_user_id == params.owner_user_id)
        }
    }
}

fn accessible_amount(
    account_type: AccountType,
    amount: &BigDecimal,
    params: &ComputeParams,
) -> BigDecimal {
    let policy = &params.policy;
    let penalty_applies = params
        .owner_age
        .map(|age| age < policy.early_withdrawal_before_age)
        .unwrap_or(false);

    let costs = match account_type {
        AccountType::Retirement401k | AccountType::TraditionalIra => {
            let mut costs = amount * &policy.tax_rate;
            if penalty_applies {
                costs += amount * &policy.early_withdrawal_penalty_rate;
            }
            costs
        }
        AccountType::RothIra => {
            // Roth contributions are post-tax; only the penalty applies.
            if penalty_applies {
                amount * &policy.early_withdrawal_penalty_rate
            } else {
                BigDecimal::from(0)
            }
        }
        AccountType::RealEstate => amount * &policy.selling_cost_rate + &policy.fixed_costs,
        // Cash, deposits, brokerage (basis unknown), and other assets are
        // treated as fully accessible for v1.
        _ => BigDecimal::from(0),
    };

    let net = amount - costs;
    if net < 0 {
        BigDecimal::from(0)
    } else {
        net
    }
}

fn to_rows(values: BTreeMap<String, BigDecimal>, currency: &Currency) -> Vec<BreakdownRow> {
    values
        .into_iter()
        .map(|(key, amount)| BreakdownRow {
            key,
            amount: Money::new(amount, currency.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::BalanceSource;
    use crate::money::Money;
    use std::str::FromStr;
    use time::macros::date;

    fn account(
        name: &str,
        account_type: AccountType,
        scope: OwnershipScope,
        owner: Option<Uuid>,
    ) -> Account {
        Account {
            id: Uuid::new_v4(),
            name: name.to_string(),
            account_type,
            institution: None,
            currency: Currency::usd(),
            ownership_scope: scope,
            owner_user_id: owner,
            archived: false,
        }
    }

    fn balance(account: &Account, amount: &str) -> Balance {
        Balance {
            id: Uuid::new_v4(),
            account_id: account.id,
            as_of: date!(2026 - 10 - 17),
            amount: Money::parse(amount, "USD").unwrap(),
            source: BalanceSource::Manual,
        }
    }

    fn params() -> ComputeParams {
        ComputeParams {
            as_of: date!(2026 - 10 - 17),
            scope: AggregationScope::Household,
            owner_user_id: None,
            owner_age: Some(35),
            policy: RedemptionPolicy::default(),
            currency: Currency::usd(),
        }
    }

    fn money(value: &str) -> Money {
        Money::parse(value, "USD").unwrap()
    }

    #[test]
    fn total_is_assets_minus_liabilities() {
        let cash = account("Cash", AccountType::Cash, OwnershipScope::Household, None);
        let mortgage = account(
            "Mortgage",
            AccountType::Mortgage,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![cash.clone(), mortgage.clone()];
        let balances = vec![balance(&cash, "500000.00"), balance(&mortgage, "200000.00")];

        let result = compute_net_worth(&accounts, &balances, &params());
        assert_eq!(result.total, money("300000.00"));
        assert_eq!(result.account_count, 2);
    }

    #[test]
    fn liquid_includes_retirement_but_excludes_real_estate() {
        let cash = account("Cash", AccountType::Cash, OwnershipScope::Household, None);
        let brokerage = account(
            "Brokerage",
            AccountType::TaxableBrokerage,
            OwnershipScope::Household,
            None,
        );
        let retirement = account(
            "401k",
            AccountType::Retirement401k,
            OwnershipScope::Household,
            None,
        );
        let home = account(
            "Home",
            AccountType::RealEstate,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![
            cash.clone(),
            brokerage.clone(),
            retirement.clone(),
            home.clone(),
        ];
        let balances = vec![
            balance(&cash, "10000.00"),
            balance(&brokerage, "40000.00"),
            balance(&retirement, "100000.00"),
            balance(&home, "500000.00"),
        ];

        let result = compute_net_worth(&accounts, &balances, &params());
        assert_eq!(result.total, money("650000.00"));
        // Cash + brokerage + retirement; the home is excluded.
        assert_eq!(result.liquid, money("150000.00"));
        // Liquidity is cash on hand only.
        assert_eq!(result.liquidity, money("10000.00"));
    }

    #[test]
    fn liquid_excludes_debt_secured_by_an_excluded_asset() {
        let cash = account(
            "Checking",
            AccountType::Checking,
            OwnershipScope::Household,
            None,
        );
        let home = account(
            "Home",
            AccountType::RealEstate,
            OwnershipScope::Household,
            None,
        );
        let mortgage = account(
            "Mortgage",
            AccountType::Mortgage,
            OwnershipScope::Household,
            None,
        );

        let accounts = vec![cash.clone(), home.clone(), mortgage.clone()];
        let balances = vec![
            balance(&cash, "10000.00"),
            balance(&home, "500000.00"),
            balance(&mortgage, "400000.00"),
        ];

        let result = compute_net_worth(&accounts, &balances, &params());
        assert_eq!(result.total, money("110000.00"));
        // The mortgage is illiquid debt, so it does not reduce liquid net worth.
        assert_eq!(result.liquid, money("10000.00"));
        assert_eq!(result.liquidity, money("10000.00"));
        // Accessible net worth still subtracts it: 10k + 470k - 400k.
        assert_eq!(result.accessible, money("80000.00"));
    }

    #[test]
    fn accessible_discounts_retirement_accounts_below_penalty_age() {
        let retirement = account(
            "401k",
            AccountType::Retirement401k,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![retirement.clone()];
        let balances = vec![balance(&retirement, "100000.00")];

        let mut p = params();
        p.owner_age = Some(35);
        let result = compute_net_worth(&accounts, &balances, &p);
        // 20% tax + 10% penalty = 70,000.
        assert_eq!(result.accessible, money("70000.00"));
    }

    #[test]
    fn no_early_penalty_at_or_after_threshold_age() {
        let retirement = account(
            "401k",
            AccountType::Retirement401k,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![retirement.clone()];
        let balances = vec![balance(&retirement, "100000.00")];

        let mut p = params();
        p.owner_age = Some(65);
        let result = compute_net_worth(&accounts, &balances, &p);
        // Only the 20% tax remains.
        assert_eq!(result.accessible, money("80000.00"));
    }

    #[test]
    fn roth_ira_has_no_tax_but_keeps_the_penalty() {
        let roth = account(
            "Roth",
            AccountType::RothIra,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![roth.clone()];
        let balances = vec![balance(&roth, "100000.00")];

        let result = compute_net_worth(&accounts, &balances, &params());
        assert_eq!(result.accessible, money("90000.00"));
    }

    #[test]
    fn accessible_discounts_real_estate_selling_costs() {
        let home = account(
            "Home",
            AccountType::RealEstate,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![home.clone()];
        let balances = vec![balance(&home, "500000.00")];

        let result = compute_net_worth(&accounts, &balances, &params());
        // 6% selling costs.
        assert_eq!(result.accessible, money("470000.00"));
    }

    #[test]
    fn personal_scope_includes_shared_but_not_other_owners() {
        let owner_a = Uuid::new_v4();
        let owner_b = Uuid::new_v4();

        let shared = account(
            "Joint",
            AccountType::Savings,
            OwnershipScope::Household,
            None,
        );
        let private_a = account(
            "A private",
            AccountType::Savings,
            OwnershipScope::User,
            Some(owner_a),
        );
        let private_b = account(
            "B private",
            AccountType::Savings,
            OwnershipScope::User,
            Some(owner_b),
        );

        let accounts = vec![shared.clone(), private_a.clone(), private_b.clone()];
        let balances = vec![
            balance(&shared, "100.00"),
            balance(&private_a, "20.00"),
            balance(&private_b, "999.00"),
        ];

        let mut p = params();
        p.scope = AggregationScope::Personal;
        p.owner_user_id = Some(owner_a);

        let result = compute_net_worth(&accounts, &balances, &p);
        // Shared once + A private, never B private.
        assert_eq!(result.total, money("120.00"));
        assert_eq!(result.account_count, 2);
    }

    #[test]
    fn household_counts_a_shared_account_once() {
        let shared = account(
            "Joint",
            AccountType::Savings,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![shared.clone()];
        let balances = vec![balance(&shared, "100.00")];

        let result = compute_net_worth(&accounts, &balances, &params());
        assert_eq!(result.total, money("100.00"));
        assert_eq!(result.account_count, 1);
    }

    #[test]
    fn ignores_balances_after_the_valuation_date() {
        let cash = account("Cash", AccountType::Cash, OwnershipScope::Household, None);
        let mut old = balance(&cash, "100.00");
        old.as_of = date!(2026 - 10 - 01);
        let mut future = balance(&cash, "999.00");
        future.as_of = date!(2026 - 12 - 01);

        let result = compute_net_worth(&[cash], &[old, future], &params());
        assert_eq!(result.total, money("100.00"));
    }

    #[test]
    fn breaks_down_by_type_and_asset_class() {
        let cash = account(
            "Cash",
            AccountType::Checking,
            OwnershipScope::Household,
            None,
        );
        let home = account(
            "Home",
            AccountType::RealEstate,
            OwnershipScope::Household,
            None,
        );
        let accounts = vec![cash.clone(), home.clone()];
        let balances = vec![balance(&cash, "1000.00"), balance(&home, "500000.00")];

        let result = compute_net_worth(&accounts, &balances, &params());
        assert_eq!(result.by_account_type.len(), 2);
        assert_eq!(result.by_asset_class.len(), 2);
        assert!(result
            .by_asset_class
            .iter()
            .any(|row| row.key == "cash" && row.amount == money("1000.00")));
    }

    #[test]
    fn flags_manual_and_stale_data() {
        let cash = account("Cash", AccountType::Cash, OwnershipScope::Household, None);
        let mut old_balance = balance(&cash, "100.00");
        old_balance.as_of = date!(2026 - 01 - 01);

        let result = compute_net_worth(&[cash], &[old_balance], &params());
        assert!(result.has_manual_data);
        assert!(result.has_stale_data);
    }

    #[test]
    fn policy_rates_are_exact_decimals() {
        let policy = RedemptionPolicy::default();
        assert_eq!(policy.tax_rate, BigDecimal::from_str("0.2").unwrap());
        assert_eq!(
            policy.early_withdrawal_penalty_rate,
            BigDecimal::from_str("0.1").unwrap()
        );
        assert_eq!(
            policy.selling_cost_rate,
            BigDecimal::from_str("0.06").unwrap()
        );
    }
}
