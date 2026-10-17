//! In-memory account and net worth repositories.
//!
//! Used by tests and development runs that do not want a database. Mimics the
//! Postgres semantics, including per-day balance upserts and immutable
//! snapshots.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use networth_application::ports::{AccountRepository, NetWorthRepository, RepositoryError};
use networth_domain::{
    Account, Balance, NewAccount, NewBalance, NewSnapshot, Snapshot, SnapshotFilter,
};
use time::Date;
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
