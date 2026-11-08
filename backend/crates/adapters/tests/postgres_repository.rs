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

#[sqlx::test(migrations = "../../migrations")]
async fn persists_securities_holdings_prices_and_fx(pool: PgPool) {
    use bigdecimal::BigDecimal;
    use networth_application::ports::{FxRepository, InvestmentRepository, PriceRepository};
    use networth_domain::{
        BasisQuality, LotSource, NewLot, NewSecurity, PriceSource, Quantity, Sector,
        SecurityAssetClass,
    };
    use std::str::FromStr;
    use time::OffsetDateTime;

    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Brokerage", AccountType::TaxableBrokerage))
        .await
        .unwrap();

    let security = repo
        .create_security(&NewSecurity {
            symbol: "VTI".to_string(),
            name: "Total Market".to_string(),
            asset_class: SecurityAssetClass::Equity,
            sector: Some(Sector::Other),
            currency: Currency::usd(),
        })
        .await
        .unwrap();

    let holding = repo
        .append_lots(
            account.id,
            security.id,
            &[NewLot {
                acquired_on: date!(2026 - 01 - 01),
                quantity: Quantity::parse("10").unwrap(),
                cost_basis: Money::parse("1000.00", "USD").unwrap(),
                fees: None,
                source: LotSource::Purchase,
                basis_quality: BasisQuality::Actual,
                basis_note: None,
            }],
        )
        .await
        .unwrap();

    assert_eq!(holding.quantity().to_string(), "10");
    assert_eq!(holding.cost_basis().amount_string(), "1000.00");
    assert_eq!(repo.list_holdings(account.id).await.unwrap().len(), 1);
    assert_eq!(repo.list_all_holdings().await.unwrap().len(), 1);

    repo.insert_price(
        "VTI",
        &Money::parse("150.00", "USD").unwrap(),
        OffsetDateTime::now_utc(),
        PriceSource::Manual,
    )
    .await
    .unwrap();

    let price = repo.latest_price("vti").await.unwrap().expect("price");
    assert_eq!(price.price.amount_string(), "150.00");
    assert_eq!(price.source, PriceSource::Manual);

    let eur = Currency::new("EUR").unwrap();
    repo.insert_rate(
        &Currency::usd(),
        &eur,
        &BigDecimal::from_str("0.92").unwrap(),
        OffsetDateTime::now_utc(),
    )
    .await
    .unwrap();

    let rate = repo
        .latest_rate(&Currency::usd(), &eur)
        .await
        .unwrap()
        .expect("rate");
    assert_eq!(rate.rate, BigDecimal::from_str("0.92").unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn records_fifo_sales_and_keeps_lot_history(pool: PgPool) {
    use networth_application::ports::InvestmentRepository;
    use networth_domain::{
        allocate_fifo, BasisQuality, Currency, LotSource, NewLot, NewSale, NewSecurity, Quantity,
        SaleMethod, SecurityAssetClass,
    };

    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Brokerage", AccountType::TaxableBrokerage))
        .await
        .unwrap();
    let security = repo
        .create_security(&NewSecurity {
            symbol: "VTI".to_string(),
            name: "Total Market".to_string(),
            asset_class: SecurityAssetClass::Equity,
            sector: None,
            currency: Currency::usd(),
        })
        .await
        .unwrap();

    let holding = repo
        .append_lots(
            account.id,
            security.id,
            &[
                NewLot {
                    acquired_on: date!(2026 - 01 - 01),
                    quantity: Quantity::parse("10").unwrap(),
                    cost_basis: Money::parse("1000.00", "USD").unwrap(),
                    fees: None,
                    source: LotSource::Purchase,
                    basis_quality: BasisQuality::Actual,
                    basis_note: None,
                },
                NewLot {
                    acquired_on: date!(2026 - 02 - 01),
                    quantity: Quantity::parse("5").unwrap(),
                    cost_basis: Money::parse("600.00", "USD").unwrap(),
                    fees: None,
                    source: LotSource::Purchase,
                    basis_quality: BasisQuality::Estimated,
                    basis_note: Some("backfilled".to_string()),
                },
            ],
        )
        .await
        .unwrap();

    let sale = NewSale {
        disposed_on: date!(2026 - 06 - 01),
        quantity: Quantity::parse("12").unwrap(),
        gross_proceeds: Money::parse("1800.00", "USD").unwrap(),
        fees: Money::parse("12.00", "USD").unwrap(),
        currency: Currency::usd(),
        method: SaleMethod::Fifo,
    };
    let allocations = allocate_fifo(&holding.lots, &sale).unwrap();
    let recorded = repo
        .record_sale(holding.id, &sale, &allocations)
        .await
        .unwrap();

    assert_eq!(recorded.realized_gain().amount_string(), "548.00");
    assert_eq!(recorded.disposals.len(), 2);
    assert_eq!(recorded.disposals[1].basis_quality, BasisQuality::Estimated);

    let remaining = repo.list_holdings(account.id).await.unwrap();
    assert_eq!(remaining[0].quantity().to_string(), "3");
    assert_eq!(remaining[0].cost_basis().amount_string(), "360.00");
    assert_eq!(remaining[0].lots.len(), 2);
    assert_eq!(remaining[0].lots[0].remaining_quantity.to_string(), "0");
    assert_eq!(remaining[0].lots[1].remaining_quantity.to_string(), "3");

    let sales = repo.list_sales(account.id).await.unwrap();
    assert_eq!(sales.len(), 1);
    assert_eq!(sales[0].realized_gain().amount_string(), "548.00");

    let error = repo.delete_holding(holding.id).await.unwrap_err();
    assert!(matches!(
        error,
        networth_application::ports::RepositoryError::Conflict(_)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_crypto_assets_wallets_and_balances(pool: PgPool) {
    use networth_application::ports::CryptoRepository;
    use networth_domain::{
        Chain, CryptoBalanceSource, CryptoWalletKind, Currency, NewCryptoAsset, NewCryptoBalance,
        NewCryptoWallet, OwnershipScope, Quantity,
    };

    let repo = PostgresAccountRepository::new(pool);
    let asset = repo
        .create_asset(&NewCryptoAsset {
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            chain: Chain::Bitcoin,
            contract_address: None,
            decimals: 18,
            currency: Currency::usd(),
            price_provider_id: Some("btc-bitcoin".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(asset.symbol, "BTC");

    let wallet = repo
        .create_wallet(&NewCryptoWallet {
            asset_id: asset.id,
            kind: CryptoWalletKind::Manual,
            chain: Chain::Bitcoin,
            address: None,
            label: "Coinbase".to_string(),
            ownership_scope: OwnershipScope::Household,
            owner_user_id: None,
        })
        .await
        .unwrap();
    assert_eq!(wallet.asset.symbol, "BTC");

    for (date, quantity, source) in [
        (date!(2026 - 01 - 01), "0.5", CryptoBalanceSource::Chain),
        (date!(2026 - 01 - 01), "0.75", CryptoBalanceSource::Manual),
        (date!(2026 - 02 - 01), "0.25", CryptoBalanceSource::Chain),
    ] {
        repo.upsert_crypto_balance(&NewCryptoBalance {
            wallet_id: wallet.id,
            as_of: date,
            quantity: Quantity::parse(quantity).unwrap(),
            source,
            provider: Some("test".to_string()),
            fetched_at: None,
        })
        .await
        .unwrap();
    }

    let latest = CryptoRepository::latest_balances(&repo, date!(2026 - 01 - 15))
        .await
        .unwrap();
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].quantity.to_string(), "0.75");
    assert_eq!(latest[0].source, CryptoBalanceSource::Manual);

    let latest = CryptoRepository::latest_balances(&repo, date!(2026 - 03 - 01))
        .await
        .unwrap();
    assert_eq!(latest[0].quantity.to_string(), "0.25");
    assert_eq!(latest[0].source, CryptoBalanceSource::Chain);

    let dust = repo
        .upsert_crypto_balance(&NewCryptoBalance {
            wallet_id: wallet.id,
            as_of: date!(2026 - 03 - 02),
            quantity: Quantity::parse("0.000000000000000001").unwrap(),
            source: CryptoBalanceSource::Manual,
            provider: None,
            fetched_at: None,
        })
        .await
        .unwrap();
    assert_eq!(dust.quantity.to_string(), "0.000000000000000001");
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_budget_income_plan_and_spending(pool: PgPool) {
    use networth_application::ports::BudgetRepository;
    use networth_domain::{
        BudgetAllocation, BudgetCategory, Currency, Money, NewBudgetIncome, NewBudgetSpending,
    };

    let repo = PostgresAccountRepository::new(pool);
    let user = repo
        .create_user(&networth_domain::NewUser {
            username: "budget-user".to_string(),
            display_name: "Budget User".to_string(),
            password_hash: "hash".to_string(),
            role: networth_domain::UserRole::Owner,
        })
        .await
        .unwrap();

    repo.upsert_income(&NewBudgetIncome {
        user_id: user.id,
        year: 2026,
        amount: Money::parse("120000.00", "USD").unwrap(),
    })
    .await
    .unwrap();

    let plan = repo
        .upsert_plan(
            user.id,
            2026,
            &Currency::usd(),
            &[
                BudgetAllocation {
                    category: BudgetCategory::FixedCosts,
                    percentage: "50".parse().unwrap(),
                },
                BudgetAllocation {
                    category: BudgetCategory::Savings,
                    percentage: "20".parse().unwrap(),
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(plan.allocations.len(), 2);
    assert_eq!(plan.total_percentage().to_string(), "70");

    repo.upsert_spending(&NewBudgetSpending {
        user_id: user.id,
        year: 2026,
        month: 1,
        category: BudgetCategory::FixedCosts,
        amount: Money::parse("1000.00", "USD").unwrap(),
        note: None,
    })
    .await
    .unwrap();

    let loaded = repo.plan(user.id, 2026).await.unwrap().expect("plan");
    assert_eq!(loaded.allocations.len(), 2);
    let spending = repo.list_spending(user.id, 2026).await.unwrap();
    assert_eq!(spending.len(), 1);
    assert_eq!(spending[0].amount.amount_string(), "1000.00");
    assert_eq!(
        repo.income(user.id, 2026)
            .await
            .unwrap()
            .expect("income")
            .amount
            .amount_string(),
        "120000.00"
    );
    assert_eq!(repo.list_incomes().await.unwrap().len(), 1);
    assert_eq!(repo.list_plans().await.unwrap().len(), 1);
    assert_eq!(repo.list_all_spending().await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_aggregation_provenance(pool: PgPool) {
    use networth_application::ports::AggregationRepository;
    use networth_domain::{AccountType, Currency, Money, Quantity};
    use time::OffsetDateTime;

    let repo = PostgresAccountRepository::new(pool);
    let account = repo
        .insert_account(&account("Linked Brokerage", AccountType::TaxableBrokerage))
        .await
        .unwrap();

    let config = repo
        .save_config("fake", true, Some("ciphertext"), Some("nonce"))
        .await
        .unwrap();
    assert!(config.enabled);
    assert_eq!(config.token_ciphertext.as_deref(), Some("ciphertext"));

    let link = repo
        .upsert_link(
            "fake",
            "provider-1",
            account.id,
            "Linked Brokerage",
            AccountType::TaxableBrokerage,
            &Currency::usd(),
        )
        .await
        .unwrap();

    let now = OffsetDateTime::now_utc();
    AggregationRepository::upsert_balance(
        &repo,
        link.id,
        now.date(),
        &Money::parse("12345.67", "USD").unwrap(),
        now,
    )
    .await
    .unwrap();
    repo.upsert_holding(
        link.id,
        "VTI",
        &Quantity::parse("10").unwrap(),
        &Money::parse("1500.00", "USD").unwrap(),
        now,
    )
    .await
    .unwrap();
    repo.touch_link(link.id, now).await.unwrap();

    let loaded = repo.config().await.unwrap().expect("config");
    assert_eq!(loaded.provider, "fake");
    assert_eq!(repo.links().await.unwrap().len(), 1);
    assert_eq!(
        AggregationRepository::latest_balance(&repo, link.id)
            .await
            .unwrap()
            .expect("balance")
            .amount
            .amount_string(),
        "12345.67"
    );
    assert_eq!(repo.holdings(link.id).await.unwrap()[0].symbol, "VTI");
    assert_eq!(repo.balances(link.id).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_crypto_lots_and_fifo_sales(pool: PgPool) {
    use networth_application::ports::CryptoRepository;
    use networth_domain::{
        allocate_fifo, BasisQuality, Chain, CryptoWalletKind, Currency, LotSource, Money,
        NewCryptoAsset, NewCryptoWallet, NewLot, NewSale, OwnershipScope, Quantity, SaleMethod,
    };

    let repo = PostgresAccountRepository::new(pool);
    let asset = repo
        .create_asset(&NewCryptoAsset {
            symbol: "BTC".to_string(),
            name: "Bitcoin".to_string(),
            chain: Chain::Bitcoin,
            contract_address: None,
            decimals: 18,
            currency: Currency::usd(),
            price_provider_id: None,
        })
        .await
        .unwrap();
    let wallet = repo
        .create_wallet(&NewCryptoWallet {
            asset_id: asset.id,
            kind: CryptoWalletKind::Manual,
            chain: Chain::Bitcoin,
            address: None,
            label: "Tax wallet".to_string(),
            ownership_scope: OwnershipScope::Household,
            owner_user_id: None,
        })
        .await
        .unwrap();

    let lots = repo
        .append_crypto_lots(
            wallet.id,
            asset.id,
            &[
                NewLot {
                    acquired_on: date!(2026 - 01 - 01),
                    quantity: Quantity::parse("10").unwrap(),
                    cost_basis: Money::parse("1000.00", "USD").unwrap(),
                    fees: None,
                    source: LotSource::Purchase,
                    basis_quality: BasisQuality::Actual,
                    basis_note: None,
                },
                NewLot {
                    acquired_on: date!(2026 - 02 - 01),
                    quantity: Quantity::parse("5").unwrap(),
                    cost_basis: Money::parse("600.00", "USD").unwrap(),
                    fees: None,
                    source: LotSource::Purchase,
                    basis_quality: BasisQuality::Estimated,
                    basis_note: Some("backfilled".to_string()),
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(lots.len(), 2);

    let sale = NewSale {
        disposed_on: date!(2026 - 06 - 01),
        quantity: Quantity::parse("12").unwrap(),
        gross_proceeds: Money::parse("1800.00", "USD").unwrap(),
        fees: Money::parse("12.00", "USD").unwrap(),
        currency: Currency::usd(),
        method: SaleMethod::Fifo,
    };
    let allocations = allocate_fifo(&lots, &sale).unwrap();
    let recorded = repo
        .record_crypto_sale(wallet.id, asset.id, &sale, &allocations)
        .await
        .unwrap();
    assert_eq!(recorded.realized_gain().amount_string(), "548.00");
    assert_eq!(recorded.disposals.len(), 2);

    let remaining = repo.list_crypto_lots(wallet.id).await.unwrap();
    assert_eq!(remaining[0].remaining_quantity.to_string(), "0");
    assert_eq!(remaining[1].remaining_quantity.to_string(), "3");
    assert_eq!(remaining[1].basis_quality, BasisQuality::Estimated);
    assert_eq!(repo.list_crypto_sales(wallet.id).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn persists_document_metadata_audit_and_provenance(pool: PgPool) {
    use networth_application::ports::{DocumentRepository, MigrationRepository};
    use networth_application::NewDocument;
    use uuid::Uuid;

    let repo = PostgresAccountRepository::new(pool);
    let id = Uuid::new_v4();
    let document = repo
        .insert(&NewDocument {
            id,
            sha256: "a".repeat(64),
            original_name: "fidelity.csv".to_string(),
            mime_type: "text/csv".to_string(),
            byte_size: 42,
            uploader_user_id: None,
            tags: vec!["retirement".to_string()],
            institution: Some("Fidelity".to_string()),
            document_type: Some("holdings".to_string()),
            classification_confidence: Some(0.9),
            parser_version: Some("generic-1".to_string()),
            status: "parsed".to_string(),
            period: Some("2026".to_string()),
            parse_error: None,
            nonce: "bm9uY2U=".to_string(),
            key_version: "1".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(document.original_name, "fidelity.csv");
    assert_eq!(
        repo.find_by_sha256(&"a".repeat(64))
            .await
            .unwrap()
            .expect("document")
            .id,
        id
    );

    let updated = repo
        .update_tags(id, &["confirmed".to_string()])
        .await
        .unwrap()
        .expect("tags");
    assert_eq!(updated.tags, vec!["confirmed".to_string()]);
    let classified = repo
        .update_classification(
            id,
            Some("Fidelity"),
            Some("holdings"),
            Some(0.99),
            "confirmed",
            Some("generic-1"),
            Some("2026"),
            None,
        )
        .await
        .unwrap()
        .expect("classification");
    assert_eq!(classified.status, "confirmed");

    repo.record_audit(id, None, "upload", r#"{"ok":true}"#)
        .await
        .unwrap();
    let audit = repo.list_audit(id).await.unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].metadata, r#"{"ok": true}"#);

    let migration_id = repo
        .record_run("generic_csv", r#"{"rows":1}"#)
        .await
        .unwrap();
    DocumentRepository::record_provenance(&repo, id, migration_id, "migration", migration_id)
        .await
        .unwrap();

    assert!(repo.soft_delete(id).await.unwrap());
    assert!(repo.list(false).await.unwrap().is_empty());
    assert_eq!(repo.list(true).await.unwrap().len(), 1);
}
