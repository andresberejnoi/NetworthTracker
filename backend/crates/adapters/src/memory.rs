//! In-memory account and net worth repositories.
//!
//! Used by tests and development runs that do not want a database. Mimics the
//! Postgres semantics, including per-day balance upserts and immutable
//! snapshots.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_application::ports::{
    AccountRepository, AggregationBalance, AggregationConfig, AggregationHolding, AggregationLink,
    AggregationRepository, BudgetRepository, CryptoRepository, FxRepository, IdentityRepository,
    InvestmentRepository, NetWorthRepository, PriceRepository, RepositoryError, StoredPrice,
    StoredRate, UserWithHash,
};
use networth_domain::{
    Account, AccountType, Balance, BudgetAllocation, BudgetIncome, BudgetSpending, CryptoAsset,
    CryptoBalance, CryptoBalanceSource, CryptoWallet, Currency, Disposal, DisposalAllocation,
    Holding, Lot, Money, NewAccount, NewBalance, NewBudgetIncome, NewBudgetSpending,
    NewCryptoAsset, NewCryptoBalance, NewCryptoWallet, NewLot, NewSale, NewSecurity, NewSnapshot,
    NewUser, PriceSource, Quantity, Sale, Security, Snapshot, SnapshotFilter, SpendingPlan, User,
    UserRole,
};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// A non-persistent account repository and net worth store.
#[derive(Default)]
pub struct InMemoryAccountRepository {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    accounts: Vec<Account>,
    balances: Vec<Balance>,
    snapshots: Vec<Snapshot>,
    users: Vec<UserWithHash>,
    sessions: Vec<SessionEntry>,
    securities: Vec<Security>,
    holdings: Vec<Holding>,
    sales: Vec<Sale>,
    crypto_assets: Vec<CryptoAsset>,
    crypto_wallets: Vec<CryptoWallet>,
    crypto_balances: Vec<CryptoBalance>,
    budget_incomes: Vec<BudgetIncome>,
    budget_plans: Vec<SpendingPlan>,
    budget_spending: Vec<BudgetSpending>,
    aggregation_config: Option<AggregationConfig>,
    aggregation_links: Vec<AggregationLink>,
    aggregation_balances: Vec<AggregationBalance>,
    aggregation_holdings: Vec<AggregationHolding>,
    crypto_lots: Vec<CryptoLotRecord>,
    crypto_sales: Vec<Sale>,
    next_lot_sequence: i64,
    prices: Vec<PriceRecord>,
    fx_rates: Vec<FxRateRecord>,
}

struct CryptoLotRecord {
    wallet_id: Uuid,
    lot: Lot,
}

struct SessionEntry {
    user_id: Uuid,
    token_hash: String,
    expires_at: OffsetDateTime,
}

struct PriceRecord {
    symbol: String,
    price: Money,
    as_of: OffsetDateTime,
    source: PriceSource,
}

struct FxRateRecord {
    base: Currency,
    quote: Currency,
    rate: BigDecimal,
    as_of: OffsetDateTime,
}

impl InMemoryAccountRepository {
    /// Create an empty repository.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AccountRepository for InMemoryAccountRepository {
    async fn insert_account(&self, new: &NewAccount) -> Result<Account, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let account = Account {
            id: Uuid::new_v4(),
            name: new.name.clone(),
            account_type: new.account_type,
            institution: new.institution.clone(),
            currency: new.currency.clone(),
            ownership_scope: new.ownership_scope,
            owner_user_id: new.owner_user_id,
            archived: false,
        };
        inner.accounts.push(account.clone());
        Ok(account)
    }

    async fn find_account(&self, id: Uuid) -> Result<Option<Account>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner.accounts.iter().find(|a| a.id == id).cloned())
    }

    async fn find_account_by_name(&self, name: &str) -> Result<Option<Account>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .accounts
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
            .cloned())
    }

    async fn list_accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut accounts: Vec<Account> = inner
            .accounts
            .iter()
            .filter(|a| include_archived || !a.archived)
            .cloned()
            .collect();
        accounts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(accounts)
    }

    async fn archive_account(&self, id: Uuid) -> Result<bool, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        match inner.accounts.iter_mut().find(|a| a.id == id) {
            Some(account) => {
                account.archived = true;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn upsert_balance(&self, new: &NewBalance) -> Result<Balance, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");

        if let Some(existing) = inner
            .balances
            .iter_mut()
            .find(|b| b.account_id == new.account_id && b.as_of == new.as_of)
        {
            existing.amount = new.amount.clone();
            existing.source = new.source;
            return Ok(existing.clone());
        }

        let balance = Balance {
            id: Uuid::new_v4(),
            account_id: new.account_id,
            as_of: new.as_of,
            amount: new.amount.clone(),
            source: new.source,
        };
        inner.balances.push(balance.clone());
        Ok(balance)
    }

    async fn upsert_balances(
        &self,
        balances: &[NewBalance],
    ) -> Result<Vec<Balance>, RepositoryError> {
        let mut saved = Vec::with_capacity(balances.len());
        for balance in balances {
            saved.push(AccountRepository::upsert_balance(self, balance).await?);
        }
        Ok(saved)
    }

    async fn list_balances(&self, account_id: Uuid) -> Result<Vec<Balance>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut balances: Vec<Balance> = inner
            .balances
            .iter()
            .filter(|b| b.account_id == account_id)
            .cloned()
            .collect();
        balances.sort_by_key(|balance| std::cmp::Reverse(balance.as_of));
        Ok(balances)
    }
}

#[async_trait]
impl NetWorthRepository for InMemoryAccountRepository {
    async fn latest_balances(&self, as_of: Date) -> Result<Vec<Balance>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");

        let mut latest: HashMap<Uuid, &Balance> = HashMap::new();
        for balance in &inner.balances {
            if balance.as_of > as_of {
                continue;
            }
            match latest.get(&balance.account_id) {
                Some(existing) if existing.as_of >= balance.as_of => {}
                _ => {
                    latest.insert(balance.account_id, balance);
                }
            }
        }

        Ok(latest.into_values().cloned().collect())
    }

    async fn insert_snapshot(&self, new: &NewSnapshot) -> Result<Snapshot, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let snapshot = Snapshot {
            id: Uuid::new_v4(),
            as_of: new.as_of,
            scope: new.scope,
            owner_user_id: new.owner_user_id,
            owner_age: new.owner_age,
            currency: new.currency.clone(),
            total: new.total.clone(),
            liquid: new.liquid.clone(),
            liquidity: new.liquidity.clone(),
            accessible: new.accessible.clone(),
            policy: new.policy.clone(),
        };
        inner.snapshots.push(snapshot.clone());
        Ok(snapshot)
    }

    async fn list_snapshots(
        &self,
        filter: &SnapshotFilter,
    ) -> Result<Vec<Snapshot>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut snapshots: Vec<Snapshot> = inner
            .snapshots
            .iter()
            .filter(|s| filter.scope.is_none_or(|scope| s.scope == scope))
            .filter(|s| {
                filter
                    .owner_user_id
                    .is_none_or(|owner| s.owner_user_id == Some(owner))
            })
            .cloned()
            .collect();
        snapshots.sort_by_key(|snapshot| snapshot.as_of);
        Ok(snapshots)
    }
}

#[async_trait]
impl IdentityRepository for InMemoryAccountRepository {
    async fn count_users(&self) -> Result<i64, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner.users.len() as i64)
    }

    async fn create_user(&self, new: &NewUser) -> Result<User, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if inner
            .users
            .iter()
            .any(|entry| entry.user.username.eq_ignore_ascii_case(&new.username))
        {
            return Err(RepositoryError::Conflict(
                "username already exists".to_string(),
            ));
        }

        let user = User {
            id: Uuid::new_v4(),
            username: new.username.clone(),
            display_name: new.display_name.clone(),
            role: new.role,
        };
        inner.users.push(UserWithHash {
            user: user.clone(),
            password_hash: new.password_hash.clone(),
        });
        Ok(user)
    }

    async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserWithHash>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .users
            .iter()
            .find(|entry| entry.user.username.eq_ignore_ascii_case(username))
            .cloned())
    }

    async fn find_user(&self, id: Uuid) -> Result<Option<User>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .users
            .iter()
            .find(|entry| entry.user.id == id)
            .map(|entry| entry.user.clone()))
    }

    async fn list_users(&self) -> Result<Vec<User>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut users: Vec<User> = inner.users.iter().map(|entry| entry.user.clone()).collect();
        users.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(users)
    }

    async fn update_user_role(&self, id: Uuid, role: UserRole) -> Result<User, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        match inner.users.iter_mut().find(|entry| entry.user.id == id) {
            Some(entry) => {
                entry.user.role = role;
                Ok(entry.user.clone())
            }
            None => Err(RepositoryError::Storage(format!("user not found: {id}"))),
        }
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        inner.sessions.push(SessionEntry {
            user_id,
            token_hash: token_hash.to_string(),
            expires_at,
        });
        Ok(())
    }

    async fn find_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<(User, OffsetDateTime)>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let Some(session) = inner
            .sessions
            .iter()
            .find(|session| session.token_hash == token_hash)
        else {
            return Ok(None);
        };
        let user = inner
            .users
            .iter()
            .find(|entry| entry.user.id == session.user_id)
            .map(|entry| entry.user.clone());
        Ok(user.map(|user| (user, session.expires_at)))
    }

    async fn delete_session(&self, token_hash: &str) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        inner
            .sessions
            .retain(|session| session.token_hash != token_hash);
        Ok(())
    }
}

#[async_trait]
impl InvestmentRepository for InMemoryAccountRepository {
    async fn create_security(&self, new: &NewSecurity) -> Result<Security, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if inner
            .securities
            .iter()
            .any(|security| security.symbol.eq_ignore_ascii_case(&new.symbol))
        {
            return Err(RepositoryError::Conflict(format!(
                "security '{}' already exists",
                new.symbol
            )));
        }

        let security = Security {
            id: Uuid::new_v4(),
            symbol: new.symbol.to_ascii_uppercase(),
            name: new.name.clone(),
            asset_class: new.asset_class,
            sector: new.sector,
            currency: new.currency.clone(),
        };
        inner.securities.push(security.clone());
        Ok(security)
    }

    async fn find_security_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<Security>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .securities
            .iter()
            .find(|security| security.symbol.eq_ignore_ascii_case(symbol))
            .cloned())
    }

    async fn list_securities(&self) -> Result<Vec<Security>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut securities = inner.securities.clone();
        securities.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(securities)
    }

    async fn append_lots(
        &self,
        account_id: Uuid,
        security_id: Uuid,
        lots: &[NewLot],
    ) -> Result<Holding, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let security = inner
            .securities
            .iter()
            .find(|security| security.id == security_id)
            .cloned()
            .ok_or_else(|| RepositoryError::Storage("security not found".to_string()))?;

        let holding_index = inner.holdings.iter().position(|holding| {
            holding.account_id == account_id && holding.security.id == security_id
        });

        let mut sequence = inner.next_lot_sequence;
        let new_lots: Vec<Lot> = lots
            .iter()
            .map(|lot| {
                let current = sequence;
                sequence += 1;
                Lot {
                    id: Uuid::new_v4(),
                    sequence: current,
                    acquired_on: lot.acquired_on,
                    original_quantity: lot.quantity.clone(),
                    remaining_quantity: lot.quantity.clone(),
                    original_cost_basis: lot.cost_basis.clone(),
                    remaining_cost_basis: lot.cost_basis.clone(),
                    fees: lot
                        .fees
                        .clone()
                        .unwrap_or_else(|| Money::zero(lot.cost_basis.currency().clone())),
                    source: lot.source,
                    basis_quality: lot.basis_quality,
                    basis_note: lot.basis_note.clone(),
                }
            })
            .collect();
        inner.next_lot_sequence = sequence;

        if let Some(index) = holding_index {
            inner.holdings[index].lots.extend(new_lots);
            return Ok(inner.holdings[index].clone());
        }

        let holding = Holding {
            id: Uuid::new_v4(),
            account_id,
            security,
            lots: new_lots,
        };
        inner.holdings.push(holding.clone());
        Ok(holding)
    }

    async fn list_holdings(&self, account_id: Uuid) -> Result<Vec<Holding>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .holdings
            .iter()
            .filter(|holding| holding.account_id == account_id)
            .cloned()
            .collect())
    }

    async fn list_all_holdings(&self) -> Result<Vec<Holding>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner.holdings.clone())
    }

    async fn record_sale(
        &self,
        holding_id: Uuid,
        sale: &NewSale,
        allocations: &[DisposalAllocation],
    ) -> Result<Sale, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let holding_index = inner
            .holdings
            .iter()
            .position(|holding| holding.id == holding_id)
            .ok_or_else(|| RepositoryError::Storage("holding not found".to_string()))?;
        let account_id = inner.holdings[holding_index].account_id;
        let security = inner.holdings[holding_index].security.clone();

        for allocation in allocations {
            let lot = inner.holdings[holding_index]
                .lots
                .iter_mut()
                .find(|lot| lot.id == allocation.lot_id)
                .ok_or_else(|| RepositoryError::Storage("lot not found".to_string()))?;
            lot.remaining_quantity = lot
                .remaining_quantity
                .checked_sub(&allocation.quantity)
                .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
            lot.remaining_cost_basis = lot
                .remaining_cost_basis
                .checked_sub(&allocation.cost_basis)
                .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
        }

        let sale_id = Uuid::new_v4();
        let disposals = allocations
            .iter()
            .map(|allocation| Disposal {
                id: Uuid::new_v4(),
                lot_id: allocation.lot_id,
                acquired_on: allocation.acquired_on,
                quantity: allocation.quantity.clone(),
                gross_proceeds: allocation.gross_proceeds.clone(),
                fees: allocation.fees.clone(),
                cost_basis: allocation.cost_basis.clone(),
                basis_quality: allocation.basis_quality,
            })
            .collect();

        let sale = Sale {
            id: sale_id,
            account_id,
            holding_id,
            security,
            disposed_on: sale.disposed_on,
            quantity: sale.quantity.clone(),
            gross_proceeds: sale.gross_proceeds.clone(),
            fees: sale.fees.clone(),
            currency: sale.currency.clone(),
            method: sale.method,
            disposals,
        };
        inner.sales.push(sale.clone());
        Ok(sale)
    }

    async fn list_sales(&self, account_id: Uuid) -> Result<Vec<Sale>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut sales: Vec<Sale> = inner
            .sales
            .iter()
            .filter(|sale| sale.account_id == account_id)
            .cloned()
            .collect();
        sales.sort_by(|a, b| {
            b.disposed_on
                .cmp(&a.disposed_on)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(sales)
    }

    async fn delete_holding(&self, holding_id: Uuid) -> Result<bool, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if inner.sales.iter().any(|sale| sale.holding_id == holding_id) {
            return Err(RepositoryError::Conflict(
                "cannot delete a position with recorded sales".to_string(),
            ));
        }
        let before = inner.holdings.len();
        inner.holdings.retain(|holding| holding.id != holding_id);
        Ok(inner.holdings.len() != before)
    }
}

#[async_trait]
impl AggregationRepository for InMemoryAccountRepository {
    async fn config(&self) -> Result<Option<AggregationConfig>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner.aggregation_config.clone())
    }

    async fn save_config(
        &self,
        provider: &str,
        enabled: bool,
        token_ciphertext: Option<&str>,
        token_nonce: Option<&str>,
    ) -> Result<AggregationConfig, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let config = AggregationConfig {
            provider: provider.to_string(),
            enabled,
            token_ciphertext: token_ciphertext.map(str::to_string),
            token_nonce: token_nonce.map(str::to_string),
        };
        inner.aggregation_config = Some(config.clone());
        Ok(config)
    }

    async fn links(&self) -> Result<Vec<AggregationLink>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner.aggregation_links.clone())
    }

    async fn upsert_link(
        &self,
        provider: &str,
        provider_account_id: &str,
        account_id: Uuid,
        display_name: &str,
        account_type: AccountType,
        currency: &Currency,
    ) -> Result<AggregationLink, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(link) = inner.aggregation_links.iter_mut().find(|link| {
            link.provider == provider && link.provider_account_id == provider_account_id
        }) {
            link.account_id = account_id;
            link.display_name = display_name.to_string();
            link.account_type = account_type;
            link.currency = currency.clone();
            return Ok(link.clone());
        }
        let link = AggregationLink {
            id: Uuid::new_v4(),
            provider: provider.to_string(),
            provider_account_id: provider_account_id.to_string(),
            account_id,
            display_name: display_name.to_string(),
            account_type,
            currency: currency.clone(),
            last_synced_at: None,
        };
        inner.aggregation_links.push(link.clone());
        Ok(link)
    }

    async fn upsert_balance(
        &self,
        link_id: Uuid,
        as_of: time::Date,
        amount: &Money,
        synced_at: OffsetDateTime,
    ) -> Result<AggregationBalance, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(balance) = inner
            .aggregation_balances
            .iter_mut()
            .find(|balance| balance.link_id == link_id && balance.as_of == as_of)
        {
            balance.amount = amount.clone();
            balance.synced_at = synced_at;
            return Ok(balance.clone());
        }
        let balance = AggregationBalance {
            link_id,
            as_of,
            amount: amount.clone(),
            synced_at,
        };
        inner.aggregation_balances.push(balance.clone());
        Ok(balance)
    }

    async fn upsert_holding(
        &self,
        link_id: Uuid,
        symbol: &str,
        quantity: &Quantity,
        value: &Money,
        synced_at: OffsetDateTime,
    ) -> Result<AggregationHolding, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(holding) = inner
            .aggregation_holdings
            .iter_mut()
            .find(|holding| holding.link_id == link_id && holding.symbol == symbol)
        {
            holding.quantity = quantity.clone();
            holding.value = value.clone();
            holding.synced_at = synced_at;
            return Ok(holding.clone());
        }
        let holding = AggregationHolding {
            link_id,
            symbol: symbol.to_string(),
            quantity: quantity.clone(),
            value: value.clone(),
            synced_at,
        };
        inner.aggregation_holdings.push(holding.clone());
        Ok(holding)
    }

    async fn latest_balance(
        &self,
        link_id: Uuid,
    ) -> Result<Option<AggregationBalance>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .aggregation_balances
            .iter()
            .filter(|balance| balance.link_id == link_id)
            .max_by_key(|balance| balance.as_of)
            .cloned())
    }

    async fn holdings(&self, link_id: Uuid) -> Result<Vec<AggregationHolding>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .aggregation_holdings
            .iter()
            .filter(|holding| holding.link_id == link_id)
            .cloned()
            .collect())
    }

    async fn balances(&self, link_id: Uuid) -> Result<Vec<AggregationBalance>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut balances: Vec<AggregationBalance> = inner
            .aggregation_balances
            .iter()
            .filter(|balance| balance.link_id == link_id)
            .cloned()
            .collect();
        balances.sort_by_key(|balance| balance.as_of);
        Ok(balances)
    }

    async fn touch_link(
        &self,
        link_id: Uuid,
        synced_at: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(link) = inner
            .aggregation_links
            .iter_mut()
            .find(|link| link.id == link_id)
        {
            link.last_synced_at = Some(synced_at);
        }
        Ok(())
    }
}

#[async_trait]
impl BudgetRepository for InMemoryAccountRepository {
    async fn upsert_income(&self, new: &NewBudgetIncome) -> Result<BudgetIncome, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(existing) = inner
            .budget_incomes
            .iter_mut()
            .find(|income| income.user_id == new.user_id && income.year == new.year)
        {
            existing.amount = new.amount.clone();
            return Ok(existing.clone());
        }
        let income = BudgetIncome {
            id: Uuid::new_v4(),
            user_id: new.user_id,
            year: new.year,
            amount: new.amount.clone(),
        };
        inner.budget_incomes.push(income.clone());
        Ok(income)
    }

    async fn income(
        &self,
        user_id: Uuid,
        year: i32,
    ) -> Result<Option<BudgetIncome>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .budget_incomes
            .iter()
            .find(|income| income.user_id == user_id && income.year == year)
            .cloned())
    }

    async fn list_incomes(&self) -> Result<Vec<BudgetIncome>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut incomes = inner.budget_incomes.clone();
        incomes.sort_by_key(|income| (income.user_id, income.year));
        Ok(incomes)
    }

    async fn upsert_plan(
        &self,
        user_id: Uuid,
        year: i32,
        currency: &Currency,
        allocations: &[BudgetAllocation],
    ) -> Result<SpendingPlan, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(existing) = inner
            .budget_plans
            .iter_mut()
            .find(|plan| plan.user_id == user_id && plan.year == year)
        {
            existing.currency = currency.clone();
            existing.allocations = allocations.to_vec();
            return Ok(existing.clone());
        }
        let plan = SpendingPlan {
            id: Uuid::new_v4(),
            user_id,
            year,
            currency: currency.clone(),
            allocations: allocations.to_vec(),
        };
        inner.budget_plans.push(plan.clone());
        Ok(plan)
    }

    async fn plan(
        &self,
        user_id: Uuid,
        year: i32,
    ) -> Result<Option<SpendingPlan>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .budget_plans
            .iter()
            .find(|plan| plan.user_id == user_id && plan.year == year)
            .cloned())
    }

    async fn list_plans(&self) -> Result<Vec<SpendingPlan>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut plans = inner.budget_plans.clone();
        plans.sort_by_key(|plan| (plan.user_id, plan.year));
        Ok(plans)
    }

    async fn upsert_spending(
        &self,
        new: &NewBudgetSpending,
    ) -> Result<BudgetSpending, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if let Some(existing) = inner.budget_spending.iter_mut().find(|spending| {
            spending.user_id == new.user_id
                && spending.year == new.year
                && spending.month == new.month
                && spending.category == new.category
        }) {
            existing.amount = new.amount.clone();
            existing.note = new.note.clone();
            return Ok(existing.clone());
        }
        let spending = BudgetSpending {
            id: Uuid::new_v4(),
            user_id: new.user_id,
            year: new.year,
            month: new.month,
            category: new.category,
            amount: new.amount.clone(),
            note: new.note.clone(),
        };
        inner.budget_spending.push(spending.clone());
        Ok(spending)
    }

    async fn list_spending(
        &self,
        user_id: Uuid,
        year: i32,
    ) -> Result<Vec<BudgetSpending>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut spending: Vec<BudgetSpending> = inner
            .budget_spending
            .iter()
            .filter(|spending| spending.user_id == user_id && spending.year == year)
            .cloned()
            .collect();
        spending.sort_by(|a, b| {
            a.month
                .cmp(&b.month)
                .then_with(|| a.category.as_str().cmp(b.category.as_str()))
        });
        Ok(spending)
    }

    async fn list_all_spending(&self) -> Result<Vec<BudgetSpending>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut spending = inner.budget_spending.clone();
        spending.sort_by(|a, b| {
            (a.user_id, a.year, a.month, a.category.as_str()).cmp(&(
                b.user_id,
                b.year,
                b.month,
                b.category.as_str(),
            ))
        });
        Ok(spending)
    }
}

#[async_trait]
impl CryptoRepository for InMemoryAccountRepository {
    async fn create_asset(&self, new: &NewCryptoAsset) -> Result<CryptoAsset, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if inner
            .crypto_assets
            .iter()
            .any(|asset| asset.chain == new.chain && asset.symbol.eq_ignore_ascii_case(&new.symbol))
        {
            return Err(RepositoryError::Conflict(
                "crypto asset already exists".to_string(),
            ));
        }
        let asset = CryptoAsset {
            id: Uuid::new_v4(),
            symbol: new.symbol.to_ascii_uppercase(),
            name: new.name.clone(),
            chain: new.chain,
            contract_address: new.contract_address.clone(),
            decimals: new.decimals,
            currency: new.currency.clone(),
            price_provider_id: new.price_provider_id.clone(),
        };
        inner.crypto_assets.push(asset.clone());
        Ok(asset)
    }

    async fn find_asset_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<CryptoAsset>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .crypto_assets
            .iter()
            .find(|asset| asset.symbol.eq_ignore_ascii_case(symbol))
            .cloned())
    }

    async fn list_assets(&self) -> Result<Vec<CryptoAsset>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut assets = inner.crypto_assets.clone();
        assets.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(assets)
    }

    async fn create_wallet(&self, new: &NewCryptoWallet) -> Result<CryptoWallet, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let asset = inner
            .crypto_assets
            .iter()
            .find(|asset| asset.id == new.asset_id)
            .cloned()
            .ok_or_else(|| RepositoryError::Storage("asset not found".to_string()))?;
        if let Some(address) = &new.address {
            if inner.crypto_wallets.iter().any(|wallet| {
                wallet.asset.id == new.asset_id
                    && wallet.chain == new.chain
                    && wallet.address.as_deref() == Some(address)
            }) {
                return Err(RepositoryError::Conflict(
                    "wallet address already exists".to_string(),
                ));
            }
        }
        let wallet = CryptoWallet {
            id: Uuid::new_v4(),
            asset,
            kind: new.kind,
            chain: new.chain,
            address: new.address.clone(),
            label: new.label.clone(),
            ownership_scope: new.ownership_scope,
            owner_user_id: new.owner_user_id,
            archived: false,
        };
        inner.crypto_wallets.push(wallet.clone());
        Ok(wallet)
    }

    async fn find_wallet(&self, id: Uuid) -> Result<Option<CryptoWallet>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .crypto_wallets
            .iter()
            .find(|wallet| wallet.id == id)
            .cloned())
    }

    async fn list_wallets(&self) -> Result<Vec<CryptoWallet>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut wallets = inner.crypto_wallets.clone();
        wallets.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(wallets)
    }

    async fn upsert_crypto_balance(
        &self,
        new: &NewCryptoBalance,
    ) -> Result<CryptoBalance, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        if !inner
            .crypto_wallets
            .iter()
            .any(|wallet| wallet.id == new.wallet_id)
        {
            return Err(RepositoryError::Storage("wallet not found".to_string()));
        }
        if let Some(existing) = inner.crypto_balances.iter_mut().find(|balance| {
            balance.wallet_id == new.wallet_id
                && balance.as_of == new.as_of
                && balance.source == new.source
        }) {
            existing.quantity = new.quantity.clone();
            existing.provider = new.provider.clone();
            existing.fetched_at = new.fetched_at;
            return Ok(existing.clone());
        }
        let balance = CryptoBalance {
            id: Uuid::new_v4(),
            wallet_id: new.wallet_id,
            as_of: new.as_of,
            quantity: new.quantity.clone(),
            source: new.source,
            provider: new.provider.clone(),
            fetched_at: new.fetched_at,
        };
        inner.crypto_balances.push(balance.clone());
        Ok(balance)
    }

    async fn list_balances(&self, wallet_id: Uuid) -> Result<Vec<CryptoBalance>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut balances: Vec<CryptoBalance> = inner
            .crypto_balances
            .iter()
            .filter(|balance| balance.wallet_id == wallet_id)
            .cloned()
            .collect();
        balances.sort_by(|a, b| {
            b.as_of
                .cmp(&a.as_of)
                .then_with(|| source_priority(a.source).cmp(&source_priority(b.source)))
        });
        Ok(balances)
    }

    async fn latest_balances(
        &self,
        as_of: time::Date,
    ) -> Result<Vec<CryptoBalance>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut by_wallet: HashMap<Uuid, CryptoBalance> = HashMap::new();
        for balance in &inner.crypto_balances {
            if balance.as_of > as_of {
                continue;
            }
            by_wallet
                .entry(balance.wallet_id)
                .and_modify(|existing| {
                    let newer = balance.as_of > existing.as_of
                        || (balance.as_of == existing.as_of
                            && source_priority(balance.source) < source_priority(existing.source));
                    if newer {
                        *existing = balance.clone();
                    }
                })
                .or_insert_with(|| balance.clone());
        }
        Ok(by_wallet.into_values().collect())
    }

    async fn append_crypto_lots(
        &self,
        wallet_id: Uuid,
        asset_id: Uuid,
        lots: &[NewLot],
    ) -> Result<Vec<Lot>, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let asset = inner
            .crypto_assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .cloned()
            .ok_or_else(|| RepositoryError::Storage("asset not found".to_string()))?;
        let mut sequence = inner.next_lot_sequence;
        let mut created = Vec::with_capacity(lots.len());
        for new in lots {
            let lot = Lot {
                id: Uuid::new_v4(),
                sequence,
                acquired_on: new.acquired_on,
                original_quantity: new.quantity.clone(),
                remaining_quantity: new.quantity.clone(),
                original_cost_basis: new.cost_basis.clone(),
                remaining_cost_basis: new.cost_basis.clone(),
                fees: new
                    .fees
                    .clone()
                    .unwrap_or_else(|| Money::zero(asset.currency.clone())),
                source: new.source,
                basis_quality: new.basis_quality,
                basis_note: new.basis_note.clone(),
            };
            sequence += 1;
            inner.crypto_lots.push(CryptoLotRecord {
                wallet_id,
                lot: lot.clone(),
            });
            created.push(lot);
        }
        inner.next_lot_sequence = sequence;
        Ok(created)
    }

    async fn list_crypto_lots(&self, wallet_id: Uuid) -> Result<Vec<Lot>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut lots: Vec<Lot> = inner
            .crypto_lots
            .iter()
            .filter(|record| record.wallet_id == wallet_id)
            .map(|record| record.lot.clone())
            .collect();
        lots.sort_by_key(|lot| (lot.acquired_on, lot.sequence));
        Ok(lots)
    }

    async fn record_crypto_sale(
        &self,
        wallet_id: Uuid,
        asset_id: Uuid,
        sale: &NewSale,
        allocations: &[DisposalAllocation],
    ) -> Result<Sale, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let asset = inner
            .crypto_assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .cloned()
            .ok_or_else(|| RepositoryError::Storage("asset not found".to_string()))?;
        for allocation in allocations {
            let record = inner
                .crypto_lots
                .iter_mut()
                .find(|record| record.wallet_id == wallet_id && record.lot.id == allocation.lot_id)
                .ok_or_else(|| RepositoryError::Storage("lot not found".to_string()))?;
            record.lot.remaining_quantity = record
                .lot
                .remaining_quantity
                .checked_sub(&allocation.quantity)
                .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
            record.lot.remaining_cost_basis = record
                .lot
                .remaining_cost_basis
                .checked_sub(&allocation.cost_basis)
                .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
        }
        let sale = Sale {
            id: Uuid::new_v4(),
            account_id: wallet_id,
            holding_id: wallet_id,
            security: Security {
                id: asset.id,
                symbol: asset.symbol,
                name: asset.name,
                asset_class: networth_domain::SecurityAssetClass::Other,
                sector: None,
                currency: asset.currency,
            },
            disposed_on: sale.disposed_on,
            quantity: sale.quantity.clone(),
            gross_proceeds: sale.gross_proceeds.clone(),
            fees: sale.fees.clone(),
            currency: sale.currency.clone(),
            method: sale.method,
            disposals: allocations
                .iter()
                .map(|allocation| Disposal {
                    id: Uuid::new_v4(),
                    lot_id: allocation.lot_id,
                    acquired_on: allocation.acquired_on,
                    quantity: allocation.quantity.clone(),
                    gross_proceeds: allocation.gross_proceeds.clone(),
                    fees: allocation.fees.clone(),
                    cost_basis: allocation.cost_basis.clone(),
                    basis_quality: allocation.basis_quality,
                })
                .collect(),
        };
        inner.crypto_sales.push(sale.clone());
        Ok(sale)
    }

    async fn list_crypto_sales(&self, wallet_id: Uuid) -> Result<Vec<Sale>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        let mut sales: Vec<Sale> = inner
            .crypto_sales
            .iter()
            .filter(|sale| sale.account_id == wallet_id)
            .cloned()
            .collect();
        sales.sort_by(|a, b| {
            b.disposed_on
                .cmp(&a.disposed_on)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(sales)
    }
}

fn source_priority(source: CryptoBalanceSource) -> u8 {
    match source {
        CryptoBalanceSource::Manual => 0,
        CryptoBalanceSource::Chain => 1,
        CryptoBalanceSource::Import => 2,
    }
}

#[async_trait]
impl PriceRepository for InMemoryAccountRepository {
    async fn insert_price(
        &self,
        symbol: &str,
        price: &Money,
        as_of: OffsetDateTime,
        source: PriceSource,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        inner.prices.push(PriceRecord {
            symbol: symbol.to_ascii_uppercase(),
            price: price.clone(),
            as_of,
            source,
        });
        Ok(())
    }

    async fn latest_price(&self, symbol: &str) -> Result<Option<StoredPrice>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .prices
            .iter()
            .filter(|record| record.symbol.eq_ignore_ascii_case(symbol))
            .max_by_key(|record| record.as_of)
            .map(|record| StoredPrice {
                price: record.price.clone(),
                as_of: record.as_of,
                source: record.source,
            }))
    }
}

#[async_trait]
impl FxRepository for InMemoryAccountRepository {
    async fn insert_rate(
        &self,
        base: &Currency,
        quote: &Currency,
        rate: &BigDecimal,
        as_of: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        inner.fx_rates.push(FxRateRecord {
            base: base.clone(),
            quote: quote.clone(),
            rate: rate.clone(),
            as_of,
        });
        Ok(())
    }

    async fn latest_rate(
        &self,
        base: &Currency,
        quote: &Currency,
    ) -> Result<Option<StoredRate>, RepositoryError> {
        let inner = self.inner.lock().expect("lock");
        Ok(inner
            .fx_rates
            .iter()
            .filter(|record| &record.base == base && &record.quote == quote)
            .max_by_key(|record| record.as_of)
            .map(|record| StoredRate {
                rate: record.rate.clone(),
                as_of: record.as_of,
            }))
    }
}
