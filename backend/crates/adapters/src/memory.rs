//! In-memory account repository.
//!
//! Used by tests and by development runs that do not want a database. It
//! implements exactly the same semantics as the Postgres adapter, including
//! the per-day upsert of balances.

use std::sync::Mutex;

use async_trait::async_trait;
use networth_application::ports::{AccountRepository, RepositoryError};
use networth_domain::{Account, Balance, NewAccount, NewBalance};
use uuid::Uuid;

/// A non-persistent [`AccountRepository`].
#[derive(Default)]
pub struct InMemoryAccountRepository {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    accounts: Vec<Account>,
    balances: Vec<Balance>,
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
