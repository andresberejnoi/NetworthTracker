//! Account use cases.

use std::sync::Arc;

use networth_domain::{
    Account, AccountType, Balance, BalanceSource, Currency, Money, NewAccount, NewBalance,
    OwnershipScope,
};
use time::Date;
use uuid::Uuid;

use crate::ports::{AccountRepository, RepositoryError};

/// Errors raised by account use cases.
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// The account does not exist.
    #[error("account not found: {0}")]
    NotFound(Uuid),
    /// A balance's currency does not match its account.
    #[error("currency mismatch: account uses {expected}, balance uses {actual}")]
    CurrencyMismatch {
        /// The account currency.
        expected: String,
        /// The balance currency.
        actual: String,
    },
    /// The repository failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Request to create an account.
#[derive(Debug, Clone)]
pub struct CreateAccount {
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

/// Request to record a balance.
#[derive(Debug, Clone)]
pub struct RecordBalance {
    /// The account to update.
    pub account_id: Uuid,
    /// The date the balance applies to.
    pub as_of: Date,
    /// The amount, in the account's currency.
    pub amount: Money,
    /// Where the value came from.
    pub source: BalanceSource,
}

/// Account use cases: create, read, archive, and record balances.
pub struct AccountService {
    repo: Arc<dyn AccountRepository>,
}

impl AccountService {
    /// Build the service over a repository.
    pub fn new(repo: Arc<dyn AccountRepository>) -> Self {
        Self { repo }
    }

    /// Create an account, validating and normalising its input.
    pub async fn create_account(&self, input: CreateAccount) -> Result<Account, AccountError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AccountError::Validation(
                "account name must not be empty".to_string(),
            ));
        }

        let institution = input
            .institution
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let new = NewAccount {
            name,
            account_type: input.account_type,
            institution,
            currency: input.currency,
            ownership_scope: input.ownership_scope,
            owner_user_id: input.owner_user_id,
        };

        Ok(self.repo.insert_account(&new).await?)
    }

    /// Fetch an account or fail with [`AccountError::NotFound`].
    pub async fn get_account(&self, id: Uuid) -> Result<Account, AccountError> {
        self.repo
            .find_account(id)
            .await?
            .ok_or(AccountError::NotFound(id))
    }

    /// List accounts, optionally including archived ones.
    pub async fn list_accounts(
        &self,
        include_archived: bool,
    ) -> Result<Vec<Account>, AccountError> {
        Ok(self.repo.list_accounts(include_archived).await?)
    }

    /// Archive an account, preserving its history.
    pub async fn archive_account(&self, id: Uuid) -> Result<(), AccountError> {
        if self.repo.archive_account(id).await? {
            Ok(())
        } else {
            Err(AccountError::NotFound(id))
        }
    }

    /// Record a balance for an account, rejecting a currency mismatch.
    pub async fn record_balance(&self, input: RecordBalance) -> Result<Balance, AccountError> {
        let account = self.get_account(input.account_id).await?;

        if account.currency != *input.amount.currency() {
            return Err(AccountError::CurrencyMismatch {
                expected: account.currency.to_string(),
                actual: input.amount.currency().to_string(),
            });
        }

        let new = NewBalance {
            account_id: input.account_id,
            as_of: input.as_of,
            amount: input.amount,
            source: input.source,
        };

        Ok(self.repo.upsert_balance(&new).await?)
    }

    /// List an account's balances, verifying the account exists first.
    pub async fn list_balances(&self, account_id: Uuid) -> Result<Vec<Balance>, AccountError> {
        self.get_account(account_id).await?;
        Ok(self.repo.list_balances(account_id).await?)
    }
}
