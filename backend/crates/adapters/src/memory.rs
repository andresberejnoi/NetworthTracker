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
    AccountRepository, FxRepository, IdentityRepository, InvestmentRepository, NetWorthRepository,
    PriceRepository, RepositoryError, StoredPrice, StoredRate, UserWithHash,
};
use networth_domain::{
    Account, Balance, Currency, Holding, Lot, Money, NewAccount, NewBalance, NewLot, NewSecurity,
    NewSnapshot, NewUser, PriceSource, Security, Snapshot, SnapshotFilter, User, UserRole,
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
    prices: Vec<PriceRecord>,
    fx_rates: Vec<FxRateRecord>,
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
            saved.push(self.upsert_balance(balance).await?);
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

    async fn upsert_holding(
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

        if let Some(existing) = inner
            .holdings
            .iter_mut()
            .find(|holding| holding.account_id == account_id && holding.security.id == security_id)
        {
            existing.lots = lots
                .iter()
                .map(|lot| Lot {
                    id: Uuid::new_v4(),
                    acquired_on: lot.acquired_on,
                    quantity: lot.quantity.clone(),
                    cost_basis: lot.cost_basis.clone(),
                })
                .collect();
            return Ok(existing.clone());
        }

        let holding = Holding {
            id: Uuid::new_v4(),
            account_id,
            security,
            lots: lots
                .iter()
                .map(|lot| Lot {
                    id: Uuid::new_v4(),
                    acquired_on: lot.acquired_on,
                    quantity: lot.quantity.clone(),
                    cost_basis: lot.cost_basis.clone(),
                })
                .collect(),
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

    async fn delete_holding(&self, holding_id: Uuid) -> Result<bool, RepositoryError> {
        let mut inner = self.inner.lock().expect("lock");
        let before = inner.holdings.len();
        inner.holdings.retain(|holding| holding.id != holding_id);
        Ok(inner.holdings.len() != before)
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
