//! Financial accounts, their classification, and dated balances.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::currency::Currency;
use crate::money::Money;

/// Whether an account is something owned or something owed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountClassification {
    /// An asset: its balance adds to net worth.
    Asset,
    /// A liability: its balance subtracts from net worth.
    Liability,
}

/// How quickly an asset can be turned into cash.
///
/// This is orthogonal to affordability/accessibility; see the design decision
/// D4. `Immediate` and `NearLiquid` count towards liquid net worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Liquidity {
    /// Cash or deposit accounts available now.
    Immediate,
    /// Marketable but not instant, such as a taxable brokerage account.
    NearLiquid,
    /// Retirement accounts, property, and other slow or restricted assets.
    Illiquid,
}

impl Liquidity {
    /// Whether the asset counts towards liquid net worth.
    pub fn is_liquid(self) -> bool {
        !matches!(self, Liquidity::Illiquid)
    }
}

/// Who can see and use an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipScope {
    /// Visible to every member of the household.
    Household,
    /// Private to a single user.
    User,
}

/// The supported account types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountType {
    /// Employer-sponsored 401(k).
    #[serde(rename = "401k")]
    Retirement401k,
    /// Traditional (pre-tax) individual retirement account.
    #[serde(rename = "traditional_ira")]
    TraditionalIra,
    /// Roth (post-tax) individual retirement account.
    #[serde(rename = "roth_ira")]
    RothIra,
    /// Ordinary taxable brokerage account.
    #[serde(rename = "taxable_brokerage")]
    TaxableBrokerage,
    /// Checking account.
    #[serde(rename = "checking")]
    Checking,
    /// Savings account.
    #[serde(rename = "savings")]
    Savings,
    /// Physical cash or a cash equivalent.
    #[serde(rename = "cash")]
    Cash,
    /// Real property.
    #[serde(rename = "real_estate")]
    RealEstate,
    /// Any other asset not covered above.
    #[serde(rename = "other_asset")]
    OtherAsset,
    /// A mortgage.
    #[serde(rename = "mortgage")]
    Mortgage,
    /// Any other debt, such as a loan or credit line.
    #[serde(rename = "loan")]
    Loan,
}

impl AccountType {
    /// Every account type, useful for exhaustive tests.
    pub const ALL: [AccountType; 11] = [
        AccountType::Retirement401k,
        AccountType::TraditionalIra,
        AccountType::RothIra,
        AccountType::TaxableBrokerage,
        AccountType::Checking,
        AccountType::Savings,
        AccountType::Cash,
        AccountType::RealEstate,
        AccountType::OtherAsset,
        AccountType::Mortgage,
        AccountType::Loan,
    ];

    /// Whether the type is an asset or a liability.
    pub fn classification(self) -> AccountClassification {
        use AccountType::*;
        match self {
            Retirement401k | TraditionalIra | RothIra | TaxableBrokerage | Checking | Savings
            | Cash | RealEstate | OtherAsset => AccountClassification::Asset,
            Mortgage | Loan => AccountClassification::Liability,
        }
    }

    /// How liquid the type is.
    pub fn liquidity(self) -> Liquidity {
        use AccountType::*;
        match self {
            Checking | Savings | Cash => Liquidity::Immediate,
            TaxableBrokerage => Liquidity::NearLiquid,
            Retirement401k | TraditionalIra | RothIra | RealEstate | OtherAsset | Mortgage
            | Loan => Liquidity::Illiquid,
        }
    }

    /// The stable string identifier used in storage and the API.
    pub fn as_str(self) -> &'static str {
        use AccountType::*;
        match self {
            Retirement401k => "401k",
            TraditionalIra => "traditional_ira",
            RothIra => "roth_ira",
            TaxableBrokerage => "taxable_brokerage",
            Checking => "checking",
            Savings => "savings",
            Cash => "cash",
            RealEstate => "real_estate",
            OtherAsset => "other_asset",
            Mortgage => "mortgage",
            Loan => "loan",
        }
    }
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing an [`AccountType`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown account type: {0}")]
pub struct AccountTypeParseError(pub String);

impl FromStr for AccountType {
    type Err = AccountTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|t| t.as_str() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| AccountTypeParseError(s.to_string()))
    }
}

/// A financial account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Surrogate identifier.
    pub id: Uuid,
    /// User-facing name.
    pub name: String,
    /// The account type.
    pub account_type: AccountType,
    /// Institution or custodian, if known.
    pub institution: Option<String>,
    /// The account's currency.
    pub currency: Currency,
    /// Household-visible or private to one user.
    pub ownership_scope: OwnershipScope,
    /// The user who owns a `user`-scoped account.
    pub owner_user_id: Option<Uuid>,
    /// Whether the account has been archived (closed).
    pub archived: bool,
}

impl Account {
    /// Whether this account is an asset.
    pub fn classification(&self) -> AccountClassification {
        self.account_type.classification()
    }

    /// Whether this account counts towards liquid net worth.
    pub fn is_liquid(&self) -> bool {
        self.account_type.liquidity().is_liquid()
    }
}

/// The fields required to create an account.
#[derive(Debug, Clone)]
pub struct NewAccount {
    /// User-facing name.
    pub name: String,
    /// The account type.
    pub account_type: AccountType,
    /// Institution or custodian, if known.
    pub institution: Option<String>,
    /// The account's currency.
    pub currency: Currency,
    /// Household-visible or private to one user.
    pub ownership_scope: OwnershipScope,
    /// The user who owns a `user`-scoped account.
    pub owner_user_id: Option<Uuid>,
}

/// Where a balance came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceSource {
    /// Typed or entered by a person.
    Manual,
    /// Imported from a spreadsheet.
    Import,
    /// Retrieved from an external provider.
    Provider,
}

/// A dated balance for an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balance {
    /// Surrogate identifier.
    pub id: Uuid,
    /// The account this balance belongs to.
    pub account_id: Uuid,
    /// The date the balance applies to.
    pub as_of: Date,
    /// The amount, always positive; liabilities are subtracted by classification.
    pub amount: Money,
    /// Where the value came from.
    pub source: BalanceSource,
}

/// The fields required to record a balance.
#[derive(Debug, Clone)]
pub struct NewBalance {
    /// The account this balance belongs to.
    pub account_id: Uuid,
    /// The date the balance applies to.
    pub as_of: Date,
    /// The amount, in the account's currency.
    pub amount: Money,
    /// Where the value came from.
    pub source: BalanceSource,
}

impl OwnershipScope {
    /// The stable string identifier used in storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            OwnershipScope::Household => "household",
            OwnershipScope::User => "user",
        }
    }
}

/// Errors raised when parsing an [`OwnershipScope`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid ownership scope: {0}")]
pub struct OwnershipScopeParseError(pub String);

impl FromStr for OwnershipScope {
    type Err = OwnershipScopeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "household" => Ok(OwnershipScope::Household),
            "user" => Ok(OwnershipScope::User),
            other => Err(OwnershipScopeParseError(other.to_string())),
        }
    }
}

impl BalanceSource {
    /// The stable string identifier used in storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            BalanceSource::Manual => "manual",
            BalanceSource::Import => "import",
            BalanceSource::Provider => "provider",
        }
    }
}

/// Errors raised when parsing a [`BalanceSource`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid balance source: {0}")]
pub struct BalanceSourceParseError(pub String);

impl FromStr for BalanceSource {
    type Err = BalanceSourceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "manual" => Ok(BalanceSource::Manual),
            "import" => Ok(BalanceSource::Import),
            "provider" => Ok(BalanceSource::Provider),
            other => Err(BalanceSourceParseError(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_has_a_classification_and_liquidity() {
        for account_type in AccountType::ALL {
            let _ = account_type.classification();
            let _ = account_type.liquidity();
            assert!(!account_type.as_str().is_empty());
        }
    }

    #[test]
    fn assets_and_liabilities_are_classified() {
        assert_eq!(
            AccountType::Retirement401k.classification(),
            AccountClassification::Asset
        );
        assert_eq!(
            AccountType::Mortgage.classification(),
            AccountClassification::Liability
        );
        assert_eq!(
            AccountType::Loan.classification(),
            AccountClassification::Liability
        );
    }

    #[test]
    fn liquidity_matches_the_spec() {
        use Liquidity::*;
        assert_eq!(AccountType::Checking.liquidity(), Immediate);
        assert_eq!(AccountType::Savings.liquidity(), Immediate);
        assert_eq!(AccountType::Cash.liquidity(), Immediate);
        assert_eq!(AccountType::TaxableBrokerage.liquidity(), NearLiquid);
        assert_eq!(AccountType::Retirement401k.liquidity(), Illiquid);
        assert_eq!(AccountType::RealEstate.liquidity(), Illiquid);
        assert!(AccountType::TaxableBrokerage.liquidity().is_liquid());
        assert!(!AccountType::RealEstate.liquidity().is_liquid());
    }

    #[test]
    fn round_trips_through_strings() {
        for account_type in AccountType::ALL {
            assert_eq!(
                AccountType::from_str(account_type.as_str()).unwrap(),
                account_type
            );
        }
        assert!(AccountType::from_str("nope").is_err());
    }

    #[test]
    fn serde_round_trips() {
        let json = serde_json::to_string(&AccountType::Retirement401k).unwrap();
        assert_eq!(json, "\"401k\"");
        let back: AccountType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AccountType::Retirement401k);

        let json = serde_json::to_string(&AccountType::TaxableBrokerage).unwrap();
        assert_eq!(json, "\"taxable_brokerage\"");
    }
}
