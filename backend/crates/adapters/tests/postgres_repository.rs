#![cfg(feature = "db-tests")]
//! Integration tests for the Postgres account repository.
//!
//! These require a live database. Run them with `make backend-test-db`, which
//! starts a throwaway Postgres and sets `DATABASE_URL`; each test gets its own
//! database via `#[sqlx::test]`.

use networth_adapters::PostgresAccountRepository;
use networth_application::ports::{AccountRepository, IdentityRepository, NetWorthRepository};
use networth_domain::{
    AccountType, AggregationScope, BalanceSource, Currency, Money, NewAccount, NewBalance,
    NewSnapshot, NewUser, OwnershipScope, RedemptionPolicy, SnapshotFilter, UserRole,
};
use sqlx::PgPool;
use time::macros::date;
use uuid::Uuid;

fn account(name: &str, account_type: AccountType) -> NewAccount {
    NewAccount {
        name: name.to_string(),
        account_type,
        institution: Some("Test Bank".to_string()),
        currency: Currency::usd(),
        ownership_scope: OwnershipScope::Household,
        owner_user_id: None,
    }
}

fn balance(account_id: Uuid, as_of: time::Date, amount: &str) -> NewBalance {
    NewBalance {
        account_id,
        as_of,
        amount: Money::parse(amount, "USD").unwrap(),
        source: BalanceSource::Manual,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn inserts_finds_and_lists(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);

    let checking = repo
        .insert_account(&account("Checking", AccountType::Checking))
        .await
        .unwrap();
    let brokerage = repo
        .insert_account(&account("Brokerage", AccountType::TaxableBrokerage))
        .await
        .unwrap();

    assert_eq!(checking.account_type, AccountType::Checking);
    assert_eq!(checking.currency, Currency::usd());
    assert!(!checking.archived);

    let found = repo
        .find_account(checking.id)
        .await
        .unwrap()
        .expect("found");
    assert_eq!(found.id, checking.id);
    assert_eq!(found.institution.as_deref(), Some("Test Bank"));

    assert!(repo.find_account(Uuid::new_v4()).await.unwrap().is_none());

    let all = repo.list_accounts(false).await.unwrap();
    assert_eq!(all.len(), 2);
    // Ordered by name.
    assert_eq!(all[0].name, "Brokerage");
    assert_eq!(all[1].id, checking.id);
    assert_eq!(brokerage.id, all[0].id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn archives_and_filters(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Petty cash", AccountType::Cash))
        .await
        .unwrap();

    assert!(repo.archive_account(account.id).await.unwrap());
    assert!(!repo.archive_account(Uuid::new_v4()).await.unwrap());

    assert!(repo.list_accounts(false).await.unwrap().is_empty());
    let all = repo.list_accounts(true).await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].archived);
}

#[sqlx::test(migrations = "../../migrations")]
async fn upserts_one_balance_per_day(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Savings", AccountType::Savings))
        .await
        .unwrap();

    let first = repo
        .upsert_balance(&balance(account.id, date!(2026 - 10 - 17), "100.00"))
        .await
        .unwrap();
    let second = repo
        .upsert_balance(&balance(account.id, date!(2026 - 10 - 17), "250.50"))
        .await
        .unwrap();

    assert_eq!(first.id, second.id, "same day upserts in place");
    assert_eq!(second.amount.amount_string(), "250.50");

    let balances = repo.list_balances(account.id).await.unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].amount.amount_string(), "250.50");
}

#[sqlx::test(migrations = "../../migrations")]
async fn preserves_high_precision_balances(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Wallet", AccountType::OtherAsset))
        .await
        .unwrap();

    repo.upsert_balance(&balance(
        account.id,
        date!(2026 - 10 - 17),
        "0.000000000000000001",
    ))
    .await
    .unwrap();

    let balances = repo.list_balances(account.id).await.unwrap();
    assert_eq!(balances[0].amount.amount_string(), "0.000000000000000001");
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_balances_newest_first(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Checking", AccountType::Checking))
        .await
        .unwrap();

    repo.upsert_balance(&balance(account.id, date!(2026 - 01 - 01), "10.00"))
        .await
        .unwrap();
    repo.upsert_balance(&balance(account.id, date!(2026 - 06 - 01), "20.00"))
        .await
        .unwrap();

    let balances = repo.list_balances(account.id).await.unwrap();
    assert_eq!(balances.len(), 2);
    assert_eq!(balances[0].as_of, date!(2026 - 06 - 01));
    assert_eq!(balances[1].as_of, date!(2026 - 01 - 01));
}

#[sqlx::test(migrations = "../../migrations")]
async fn rejects_a_balance_for_an_unknown_account(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let result = repo
        .upsert_balance(&balance(Uuid::new_v4(), date!(2026 - 10 - 17), "1.00"))
        .await;
    assert!(
        result.is_err(),
        "the foreign key must reject unknown accounts"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_ownership_scope_and_account_type(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let mut input = account("Private Roth", AccountType::RothIra);
    input.ownership_scope = OwnershipScope::User;
    input.institution = None;

    let created = repo.insert_account(&input).await.unwrap();
    let found = repo.find_account(created.id).await.unwrap().unwrap();

    assert_eq!(found.account_type, AccountType::RothIra);
    assert_eq!(found.ownership_scope, OwnershipScope::User);
    assert_eq!(found.institution, None);
}

#[sqlx::test(migrations = "../../migrations")]
async fn latest_balances_picks_the_most_recent_on_or_before(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Checking", AccountType::Checking))
        .await
        .unwrap();

    repo.upsert_balance(&balance(account.id, date!(2026 - 01 - 01), "10.00"))
        .await
        .unwrap();
    repo.upsert_balance(&balance(account.id, date!(2026 - 06 - 01), "20.00"))
        .await
        .unwrap();
    repo.upsert_balance(&balance(account.id, date!(2026 - 12 - 01), "30.00"))
        .await
        .unwrap();

    let latest = repo.latest_balances(date!(2026 - 10 - 17)).await.unwrap();
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].amount.amount_string(), "20.00");
}

#[sqlx::test(migrations = "../../migrations")]
async fn upserts_a_batch_in_one_transaction(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Checking", AccountType::Checking))
        .await
        .unwrap();

    let saved = repo
        .upsert_balances(&[
            balance(account.id, date!(2026 - 01 - 01), "10.00"),
            balance(account.id, date!(2026 - 02 - 01), "20.00"),
            balance(account.id, date!(2026 - 03 - 01), "30.00"),
        ])
        .await
        .unwrap();

    assert_eq!(saved.len(), 3);
    let listed = repo.list_balances(account.id).await.unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].as_of, date!(2026 - 03 - 01));
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_and_lists_snapshots(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);
    let policy = RedemptionPolicy::default();

    let new = NewSnapshot {
        as_of: date!(2026 - 10 - 17),
        scope: AggregationScope::Household,
        owner_user_id: None,
        owner_age: Some(35),
        currency: Currency::usd(),
        total: Money::parse("100.00", "USD").unwrap(),
        liquid: Money::parse("50.00", "USD").unwrap(),
        liquidity: Money::parse("25.00", "USD").unwrap(),
        accessible: Money::parse("40.00", "USD").unwrap(),
        policy: policy.clone(),
    };

    let snapshot = repo.insert_snapshot(&new).await.unwrap();
    assert_eq!(snapshot.total.amount_string(), "100.00");

    let listed = repo
        .list_snapshots(&SnapshotFilter::default())
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, snapshot.id);
    assert_eq!(listed[0].accessible.amount_string(), "40.00");
    assert_eq!(listed[0].policy, policy);
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_users_and_sessions(pool: PgPool) {
    let repo = PostgresAccountRepository::new(pool);

    assert_eq!(repo.count_users().await.unwrap(), 0);

    let user = repo
        .create_user(&NewUser {
            username: "alice".to_string(),
            display_name: "Alice".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Owner,
        })
        .await
        .unwrap();

    assert_eq!(user.role, UserRole::Owner);
    assert_eq!(repo.count_users().await.unwrap(), 1);

    let found = repo
        .find_user_by_username("ALICE")
        .await
        .unwrap()
        .expect("user");
    assert_eq!(found.user.id, user.id);
    assert_eq!(found.password_hash, "hash");

    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(1);
    repo.create_session(user.id, "token-hash", expires_at)
        .await
        .unwrap();

    let (session_user, session_expiry) = repo
        .find_session("token-hash")
        .await
        .unwrap()
        .expect("session");
    assert_eq!(session_user.id, user.id);
    assert!(session_expiry > time::OffsetDateTime::now_utc());

    repo.delete_session("token-hash").await.unwrap();
    assert!(repo.find_session("token-hash").await.unwrap().is_none());

    let updated = repo
        .update_user_role(user.id, UserRole::Viewer)
        .await
        .unwrap();
    assert_eq!(updated.role, UserRole::Viewer);

    let duplicate = repo
        .create_user(&NewUser {
            username: "Alice".to_string(),
            display_name: "Alice again".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Editor,
        })
        .await;
    assert!(
        duplicate.is_err(),
        "usernames are unique (case-insensitive)"
    );
}
