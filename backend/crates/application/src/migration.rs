//! Guided, locally processed migration from generic CSV/JSON files.
//!
//! A source adapter turns rows into normalized account, balance, holding-lot,
//! crypto-wallet, crypto-balance, and crypto-lot records. The same parser drives
//! preview and apply, so what the user previews is what is written. Apply is
//! atomic by default; partial apply is explicit. Existing accounts/securities/
//! wallets are reused by stable keys and provenance rows link written entities
//! to the migration run.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use bigdecimal::BigDecimal;
use networth_domain::{
    Account, AccountType, BalanceSource, BasisQuality, Chain, CryptoBalanceSource,
    CryptoWalletKind, Currency, LotSource, Money, NewAccount, NewBalance, NewCryptoAsset,
    NewCryptoBalance, NewCryptoWallet, NewLot, NewSecurity, OwnershipScope, Quantity,
    SecurityAssetClass,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Iso8601, Date};
use uuid::Uuid;

use crate::ports::{
    AccountRepository, CryptoRepository, InvestmentRepository, MigrationRepository, RepositoryError,
};

/// Supported generic source adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationSourceKind {
    /// Comma-separated values with a header row.
    #[serde(rename = "generic_csv")]
    GenericCsv,
    /// A JSON array of objects.
    #[serde(rename = "generic_json")]
    GenericJson,
}

impl MigrationSourceKind {
    /// Stable identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            MigrationSourceKind::GenericCsv => "generic_csv",
            MigrationSourceKind::GenericJson => "generic_json",
        }
    }
}

/// Mapping from normalized field names to source column/object keys.
///
/// Missing fields use the same default name (for example `account_name`).
pub type MigrationMapping = HashMap<String, String>;

/// How apply treats invalid rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationMode {
    /// Reject the whole file if any row is invalid.
    Atomic,
    /// Write valid rows and report the rejected rows.
    Partial,
}

/// A preview request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRequest {
    /// Source adapter.
    pub source: MigrationSourceKind,
    /// Field mapping, normalized field -> source key.
    #[serde(default)]
    pub mapping: MigrationMapping,
    /// Raw CSV text or JSON array text.
    pub content: String,
}

/// An apply request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationApplyRequest {
    /// Source and rows.
    #[serde(flatten)]
    pub request: MigrationRequest,
    /// Atomic or explicit partial apply.
    pub mode: MigrationMode,
    /// Confirms that a verified backup was taken before this request.
    pub backup_confirmed: bool,
}

/// A row-level migration error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationRowError {
    /// Source row number (1-based, including the header for CSV).
    pub row: usize,
    /// Why the row was rejected.
    pub message: String,
}

/// Dry-run summary. No writes occur.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationPreview {
    /// Source adapter.
    pub source: String,
    /// Source rows seen.
    pub total_rows: usize,
    /// Valid account rows.
    pub accounts: usize,
    /// Valid balance rows.
    pub balances: usize,
    /// Valid holding-lot rows.
    pub holding_lots: usize,
    /// Valid crypto wallet rows.
    pub crypto_wallets: usize,
    /// Valid crypto balance rows.
    pub crypto_balances: usize,
    /// Valid crypto lot rows.
    pub crypto_lots: usize,
    /// Rows that would be rejected.
    pub errors: Vec<MigrationRowError>,
}

/// The outcome of applying a migration.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    /// Migration run id, for provenance lookups.
    pub migration_id: Uuid,
    /// Accounts created.
    pub accounts_created: usize,
    /// Balances saved.
    pub balances_saved: usize,
    /// Holding lots written.
    pub holding_lots_saved: usize,
    /// Existing holdings skipped to avoid duplicate lots.
    pub holdings_skipped: usize,
    /// Crypto wallets created.
    pub crypto_wallets_created: usize,
    /// Crypto balances saved.
    pub crypto_balances_saved: usize,
    /// Crypto lots written.
    pub crypto_lots_saved: usize,
    /// Existing crypto wallets whose lots were skipped.
    pub crypto_holdings_skipped: usize,
    /// Rows rejected in partial mode.
    pub errors: Vec<MigrationRowError>,
}

/// Errors raised by guided migration.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// The request or rows are invalid.
    #[error("{0}")]
    Validation(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Parsed and validated migration rows.
#[derive(Debug, Clone)]
enum Record {
    Account {
        name: String,
        account_type: AccountType,
        institution: Option<String>,
        currency: Currency,
        ownership_scope: OwnershipScope,
        archived: bool,
    },
    Balance {
        account_name: String,
        as_of: Date,
        amount: Money,
        source: BalanceSource,
    },
    HoldingLot {
        account_name: String,
        symbol: String,
        security_name: String,
        asset_class: SecurityAssetClass,
        lot: NewLot,
    },
    CryptoWallet {
        asset_symbol: String,
        asset_name: String,
        chain: Chain,
        decimals: u32,
        label: String,
        kind: CryptoWalletKind,
        address: Option<String>,
        ownership_scope: OwnershipScope,
    },
    CryptoBalance {
        asset_symbol: String,
        asset_name: String,
        chain: Chain,
        decimals: u32,
        label: String,
        kind: CryptoWalletKind,
        address: Option<String>,
        ownership_scope: OwnershipScope,
        as_of: Date,
        quantity: Quantity,
        source: CryptoBalanceSource,
    },
    CryptoLot {
        asset_symbol: String,
        asset_name: String,
        chain: Chain,
        decimals: u32,
        label: String,
        kind: CryptoWalletKind,
        address: Option<String>,
        ownership_scope: OwnershipScope,
        lot: NewLot,
    },
}

struct RawRow {
    row: usize,
    values: HashMap<String, String>,
}

impl RawRow {
    fn get<'a>(&'a self, mapping: &'a MigrationMapping, field: &str) -> &'a str {
        let key = mapping
            .get(field)
            .map(String::as_str)
            .unwrap_or(field)
            .trim()
            .to_ascii_lowercase();
        self.values.get(&key).map(String::as_str).unwrap_or("")
    }
}

fn required(row: &RawRow, mapping: &MigrationMapping, field: &str) -> Result<String, String> {
    let value = row.get(mapping, field).trim();
    if value.is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(value.to_string())
    }
}

fn default_value(row: &RawRow, mapping: &MigrationMapping, field: &str, default: &str) -> String {
    let value = row.get(mapping, field).trim();
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

fn parse_csv(content: &str) -> Result<Vec<RawRow>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|error| error.to_string())?
        .iter()
        .map(|header| header.trim().to_ascii_lowercase())
        .collect();

    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|error| error.to_string())?;
        let mut values = HashMap::new();
        for (column, header) in headers.iter().enumerate() {
            values.insert(
                header.clone(),
                record.get(column).unwrap_or_default().trim().to_string(),
            );
        }
        rows.push(RawRow {
            row: index + 2,
            values,
        });
    }
    Ok(rows)
}

fn parse_json(content: &str) -> Result<Vec<RawRow>, String> {
    let value: serde_json::Value =
        serde_json::from_str(content).map_err(|error| format!("invalid JSON: {error}"))?;
    let array = value
        .as_array()
        .ok_or_else(|| "generic JSON must be an array of objects".to_string())?;
    let mut rows = Vec::new();
    for (index, item) in array.iter().enumerate() {
        let object = item
            .as_object()
            .ok_or_else(|| format!("row {} is not an object", index + 1))?;
        let mut values = HashMap::new();
        for (key, value) in object {
            let text = match value {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(value) => value.clone(),
                other => other.to_string(),
            };
            values.insert(key.trim().to_ascii_lowercase(), text);
        }
        rows.push(RawRow {
            row: index + 1,
            values,
        });
    }
    Ok(rows)
}

fn parse_source(source: MigrationSourceKind, content: &str) -> Result<Vec<RawRow>, String> {
    match source {
        MigrationSourceKind::GenericCsv => parse_csv(content),
        MigrationSourceKind::GenericJson => parse_json(content),
    }
}

fn parse_record(row: &RawRow, mapping: &MigrationMapping) -> Result<Record, String> {
    let record_type = required(row, mapping, "record_type")?.to_ascii_lowercase();
    let currency = Currency::new(default_value(row, mapping, "currency", "USD"))
        .map_err(|error| error.to_string())?;

    match record_type.as_str() {
        "account" => {
            let name = required(row, mapping, "account_name")?;
            let account_type =
                AccountType::from_str(&default_value(row, mapping, "account_type", "other_asset"))
                    .map_err(|error| error.to_string())?;
            let ownership_scope = OwnershipScope::from_str(&default_value(
                row,
                mapping,
                "ownership_scope",
                "household",
            ))
            .map_err(|error| error.to_string())?;
            let institution = {
                let value = row.get(mapping, "institution").trim();
                (!value.is_empty()).then(|| value.to_string())
            };
            Ok(Record::Account {
                name,
                account_type,
                institution,
                currency,
                ownership_scope,
                archived: parse_bool(default_value(row, mapping, "archived", "false").as_str())?,
            })
        }
        "balance" => {
            let account_name = required(row, mapping, "account_name")?;
            let as_of = Date::parse(row.get(mapping, "as_of"), &Iso8601::DATE)
                .map_err(|error| format!("invalid as_of date: {error}"))?;
            let amount = Money::parse(row.get(mapping, "amount"), currency.as_str())
                .map_err(|error| error.to_string())?;
            let source = BalanceSource::from_str(&default_value(row, mapping, "source", "import"))
                .map_err(|error| error.to_string())?;
            Ok(Record::Balance {
                account_name,
                as_of,
                amount,
                source,
            })
        }
        "holding_lot" => {
            let account_name = required(row, mapping, "account_name")?;
            let symbol = required(row, mapping, "symbol")?.to_ascii_uppercase();
            let security_name = default_value(row, mapping, "security_name", &symbol);
            let asset_class =
                SecurityAssetClass::from_str(&default_value(row, mapping, "asset_class", "other"))
                    .map_err(|error| error.to_string())?;
            Ok(Record::HoldingLot {
                account_name,
                symbol,
                security_name,
                asset_class,
                lot: parse_lot(row, mapping, &currency)?,
            })
        }
        "crypto_wallet" => {
            let (asset_symbol, asset_name, chain, decimals) = parse_crypto_asset(row, mapping)?;
            let (label, kind, address, ownership_scope) = parse_wallet(row, mapping)?;
            Ok(Record::CryptoWallet {
                asset_symbol,
                asset_name,
                chain,
                decimals,
                label,
                kind,
                address,
                ownership_scope,
            })
        }
        "crypto_balance" => {
            let (asset_symbol, asset_name, chain, decimals) = parse_crypto_asset(row, mapping)?;
            let (label, kind, address, ownership_scope) = parse_wallet(row, mapping)?;
            let as_of = Date::parse(row.get(mapping, "as_of"), &Iso8601::DATE)
                .map_err(|error| format!("invalid as_of date: {error}"))?;
            let quantity =
                Quantity::parse(row.get(mapping, "quantity")).map_err(|error| error.to_string())?;
            let source =
                CryptoBalanceSource::from_str(&default_value(row, mapping, "source", "import"))
                    .map_err(|error| error.to_string())?;
            Ok(Record::CryptoBalance {
                asset_symbol,
                asset_name,
                chain,
                decimals,
                label,
                kind,
                address,
                ownership_scope,
                as_of,
                quantity,
                source,
            })
        }
        "crypto_lot" => {
            let (asset_symbol, asset_name, chain, decimals) = parse_crypto_asset(row, mapping)?;
            let (label, kind, address, ownership_scope) = parse_wallet(row, mapping)?;
            Ok(Record::CryptoLot {
                asset_symbol,
                asset_name,
                chain,
                decimals,
                label,
                kind,
                address,
                ownership_scope,
                lot: parse_lot(row, mapping, &currency)?,
            })
        }
        other => Err(format!("unknown record_type '{other}'")),
    }
}

fn parse_lot(
    row: &RawRow,
    mapping: &MigrationMapping,
    currency: &Currency,
) -> Result<NewLot, String> {
    let acquired_on = Date::parse(row.get(mapping, "acquired_on"), &Iso8601::DATE)
        .map_err(|error| format!("invalid acquired_on date: {error}"))?;
    let quantity =
        Quantity::parse(row.get(mapping, "quantity")).map_err(|error| error.to_string())?;
    let cost_basis = Money::parse(row.get(mapping, "cost_basis"), currency.as_str())
        .map_err(|error| error.to_string())?;
    let fees = {
        let value = row.get(mapping, "fees").trim();
        if value.is_empty() {
            None
        } else {
            Some(Money::parse(value, currency.as_str()).map_err(|error| error.to_string())?)
        }
    };
    let source = LotSource::from_str(&default_value(row, mapping, "lot_source", "import"))
        .map_err(|error| error.to_string())?;
    let basis_quality =
        BasisQuality::from_str(&default_value(row, mapping, "basis_quality", "actual"))
            .map_err(|error| error.to_string())?;
    let basis_note = {
        let value = row.get(mapping, "basis_note").trim();
        (!value.is_empty()).then(|| value.to_string())
    };
    Ok(NewLot {
        acquired_on,
        quantity,
        cost_basis,
        fees,
        source,
        basis_quality,
        basis_note,
    })
}

fn parse_crypto_asset(
    row: &RawRow,
    mapping: &MigrationMapping,
) -> Result<(String, String, Chain, u32), String> {
    let symbol = required(row, mapping, "asset_symbol")?.to_ascii_uppercase();
    let name = default_value(row, mapping, "asset_name", &symbol);
    let chain =
        Chain::from_str(&required(row, mapping, "chain")?).map_err(|error| error.to_string())?;
    let decimals = default_value(row, mapping, "decimals", "18")
        .parse::<u32>()
        .map_err(|error| format!("invalid decimals: {error}"))?;
    Ok((symbol, name, chain, decimals))
}

fn parse_wallet(
    row: &RawRow,
    mapping: &MigrationMapping,
) -> Result<(String, CryptoWalletKind, Option<String>, OwnershipScope), String> {
    let label = required(row, mapping, "wallet_label")?;
    let kind = CryptoWalletKind::from_str(&default_value(row, mapping, "wallet_kind", "manual"))
        .map_err(|error| error.to_string())?;
    let address = {
        let value = row.get(mapping, "wallet_address").trim();
        (!value.is_empty()).then(|| value.to_string())
    };
    if kind == CryptoWalletKind::Address && address.is_none() {
        return Err("wallet_address is required for an address wallet".to_string());
    }
    let ownership_scope =
        OwnershipScope::from_str(&default_value(row, mapping, "ownership_scope", "household"))
            .map_err(|error| error.to_string())?;
    Ok((label, kind, address, ownership_scope))
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "false" | "0" | "no" => Ok(false),
        "true" | "1" | "yes" => Ok(true),
        other => Err(format!("invalid boolean '{other}'")),
    }
}

fn parse_records(request: &MigrationRequest) -> (Vec<Record>, Vec<MigrationRowError>, usize) {
    let rows = match parse_source(request.source, &request.content) {
        Ok(rows) => rows,
        Err(message) => return (Vec::new(), vec![MigrationRowError { row: 0, message }], 0),
    };
    let total = rows.len();
    let mut records = Vec::new();
    let mut errors = Vec::new();
    for row in rows {
        if row.values.values().all(|value| value.trim().is_empty()) {
            continue;
        }
        match parse_record(&row, &request.mapping) {
            Ok(record) => records.push(record),
            Err(message) => errors.push(MigrationRowError {
                row: row.row,
                message,
            }),
        }
    }
    (records, errors, total)
}

/// Guided migration use cases.
pub struct MigrationService {
    accounts: Arc<dyn AccountRepository>,
    investments: Arc<dyn InvestmentRepository>,
    crypto: Arc<dyn CryptoRepository>,
    migrations: Arc<dyn MigrationRepository>,
}

impl MigrationService {
    /// Build the service over the repositories it writes.
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        investments: Arc<dyn InvestmentRepository>,
        crypto: Arc<dyn CryptoRepository>,
        migrations: Arc<dyn MigrationRepository>,
    ) -> Self {
        Self {
            accounts,
            investments,
            crypto,
            migrations,
        }
    }

    /// Parse and validate without writing anything.
    pub async fn preview(
        &self,
        request: &MigrationRequest,
    ) -> Result<MigrationPreview, MigrationError> {
        let (records, errors, total_rows) = parse_records(request);
        let mut preview = MigrationPreview {
            source: request.source.as_str().to_string(),
            total_rows,
            accounts: 0,
            balances: 0,
            holding_lots: 0,
            crypto_wallets: 0,
            crypto_balances: 0,
            crypto_lots: 0,
            errors,
        };
        for record in records {
            match record {
                Record::Account { .. } => preview.accounts += 1,
                Record::Balance { .. } => preview.balances += 1,
                Record::HoldingLot { .. } => preview.holding_lots += 1,
                Record::CryptoWallet { .. } => preview.crypto_wallets += 1,
                Record::CryptoBalance { .. } => preview.crypto_balances += 1,
                Record::CryptoLot { .. } => preview.crypto_lots += 1,
            }
        }
        Ok(preview)
    }

    /// Apply a previously previewed request.
    pub async fn apply(
        &self,
        request: &MigrationApplyRequest,
    ) -> Result<MigrationReport, MigrationError> {
        if !request.backup_confirmed {
            return Err(MigrationError::Validation(
                "a verified backup must be taken before applying a migration".to_string(),
            ));
        }

        let (records, errors, _) = parse_records(&request.request);
        if request.mode == MigrationMode::Atomic && !errors.is_empty() {
            return Err(MigrationError::Validation(format!(
                "{} row(s) failed validation; nothing was written",
                errors.len()
            )));
        }

        let source = request.request.source.as_str();
        let summary = serde_json::json!({
            "source": source,
            "mode": match request.mode {
                MigrationMode::Atomic => "atomic",
                MigrationMode::Partial => "partial",
            },
            "errors": errors.len(),
        })
        .to_string();
        let migration_id = self.migrations.record_run(source, &summary).await?;

        let existing = self.accounts.list_accounts(true).await?;
        let mut accounts: HashMap<String, Account> = existing
            .into_iter()
            .map(|account| (account.name.to_ascii_lowercase(), account))
            .collect();
        let mut securities: HashMap<String, Uuid> = HashMap::new();
        let mut existing_holdings: HashSet<(Uuid, Uuid)> = HashSet::new();
        let mut new_holdings: HashSet<(Uuid, Uuid)> = HashSet::new();
        let mut wallets: Vec<networth_domain::CryptoWallet> = self.crypto.list_wallets().await?;
        let mut new_wallets: HashSet<Uuid> = HashSet::new();

        let mut report = MigrationReport {
            migration_id,
            accounts_created: 0,
            balances_saved: 0,
            holding_lots_saved: 0,
            holdings_skipped: 0,
            crypto_wallets_created: 0,
            crypto_balances_saved: 0,
            crypto_lots_saved: 0,
            crypto_holdings_skipped: 0,
            errors,
        };

        for record in records {
            match record {
                Record::Account {
                    name,
                    account_type,
                    institution,
                    currency,
                    ownership_scope,
                    archived,
                } => {
                    let key = name.to_ascii_lowercase();
                    if accounts.contains_key(&key) {
                        continue;
                    }
                    let created = self
                        .accounts
                        .insert_account(&NewAccount {
                            name: name.clone(),
                            account_type,
                            institution,
                            currency,
                            ownership_scope,
                            owner_user_id: None,
                        })
                        .await?;
                    if archived {
                        self.accounts.archive_account(created.id).await?;
                    }
                    self.migrations
                        .record_provenance(migration_id, "account", created.id, source)
                        .await?;
                    report.accounts_created += 1;
                    accounts.insert(key, created);
                }
                Record::Balance {
                    account_name,
                    as_of,
                    amount,
                    source: balance_source,
                } => {
                    let Some(account) = accounts.get(&account_name.to_ascii_lowercase()) else {
                        return Err(MigrationError::Validation(format!(
                            "unknown account '{account_name}'"
                        )));
                    };
                    self.accounts
                        .upsert_balance(&NewBalance {
                            account_id: account.id,
                            as_of,
                            amount,
                            source: balance_source,
                        })
                        .await?;
                    self.migrations
                        .record_provenance(migration_id, "account", account.id, source)
                        .await?;
                    report.balances_saved += 1;
                }
                Record::HoldingLot {
                    account_name,
                    symbol,
                    security_name,
                    asset_class,
                    lot,
                } => {
                    let Some(account) = accounts.get(&account_name.to_ascii_lowercase()) else {
                        return Err(MigrationError::Validation(format!(
                            "unknown account '{account_name}'"
                        )));
                    };
                    let security_id = if let Some(id) = securities.get(&symbol) {
                        *id
                    } else if let Some(security) =
                        self.investments.find_security_by_symbol(&symbol).await?
                    {
                        securities.insert(symbol.clone(), security.id);
                        security.id
                    } else {
                        let security = self
                            .investments
                            .create_security(&NewSecurity {
                                symbol: symbol.clone(),
                                name: security_name,
                                asset_class,
                                sector: None,
                                currency: lot.cost_basis.currency().clone(),
                            })
                            .await?;
                        securities.insert(symbol.clone(), security.id);
                        security.id
                    };

                    let key = (account.id, security_id);
                    let holdings = self.investments.list_holdings(account.id).await?;
                    if holdings
                        .iter()
                        .any(|holding| holding.security.id == security_id)
                    {
                        existing_holdings.insert(key);
                    }
                    if existing_holdings.contains(&key) && !new_holdings.contains(&key) {
                        report.holdings_skipped += 1;
                        continue;
                    }
                    let holding = self
                        .investments
                        .append_lots(account.id, security_id, &[lot])
                        .await?;
                    new_holdings.insert(key);
                    self.migrations
                        .record_provenance(migration_id, "holding", holding.id, source)
                        .await?;
                    report.holding_lots_saved += 1;
                }
                Record::CryptoWallet {
                    asset_symbol,
                    asset_name,
                    chain,
                    decimals,
                    label,
                    kind,
                    address,
                    ownership_scope,
                } => {
                    let asset = self
                        .ensure_crypto_asset(&asset_symbol, &asset_name, chain, decimals)
                        .await?;
                    if let Some(wallet) = wallets.iter().find(|wallet| {
                        wallet.asset.id == asset.id
                            && wallet.kind == kind
                            && if kind == CryptoWalletKind::Manual {
                                wallet.label.eq_ignore_ascii_case(&label)
                            } else {
                                wallet.address == address
                            }
                    }) {
                        let _ = wallet;
                        continue;
                    }
                    let wallet = self
                        .crypto
                        .create_wallet(&NewCryptoWallet {
                            asset_id: asset.id,
                            kind,
                            chain,
                            address,
                            label,
                            ownership_scope,
                            owner_user_id: None,
                        })
                        .await?;
                    self.migrations
                        .record_provenance(migration_id, "crypto_wallet", wallet.id, source)
                        .await?;
                    new_wallets.insert(wallet.id);
                    wallets.push(wallet);
                    report.crypto_wallets_created += 1;
                }
                Record::CryptoBalance {
                    asset_symbol,
                    asset_name,
                    chain,
                    decimals,
                    label,
                    kind,
                    address,
                    ownership_scope,
                    as_of,
                    quantity,
                    source: balance_source,
                } => {
                    let wallets_before = new_wallets.len();
                    let wallet = self
                        .ensure_wallet(
                            &asset_symbol,
                            &asset_name,
                            chain,
                            decimals,
                            &label,
                            kind,
                            address,
                            ownership_scope,
                            &mut wallets,
                            &mut new_wallets,
                            migration_id,
                            source,
                        )
                        .await?;
                    report.crypto_wallets_created += new_wallets.len() - wallets_before;
                    self.crypto
                        .upsert_crypto_balance(&NewCryptoBalance {
                            wallet_id: wallet.id,
                            as_of,
                            quantity,
                            source: balance_source,
                            provider: None,
                            fetched_at: None,
                        })
                        .await?;
                    self.migrations
                        .record_provenance(migration_id, "crypto_wallet", wallet.id, source)
                        .await?;
                    report.crypto_balances_saved += 1;
                }
                Record::CryptoLot {
                    asset_symbol,
                    asset_name,
                    chain,
                    decimals,
                    label,
                    kind,
                    address,
                    ownership_scope,
                    lot,
                } => {
                    let wallets_before = new_wallets.len();
                    let wallet = self
                        .ensure_wallet(
                            &asset_symbol,
                            &asset_name,
                            chain,
                            decimals,
                            &label,
                            kind,
                            address,
                            ownership_scope,
                            &mut wallets,
                            &mut new_wallets,
                            migration_id,
                            source,
                        )
                        .await?;
                    report.crypto_wallets_created += new_wallets.len() - wallets_before;
                    if !new_wallets.contains(&wallet.id) {
                        report.crypto_holdings_skipped += 1;
                        continue;
                    }
                    let lots = self
                        .crypto
                        .append_crypto_lots(wallet.id, wallet.asset.id, &[lot])
                        .await?;
                    for lot in lots {
                        self.migrations
                            .record_provenance(migration_id, "crypto_lot", lot.id, source)
                            .await?;
                    }
                    report.crypto_lots_saved += 1;
                }
            }
        }

        Ok(report)
    }

    async fn ensure_crypto_asset(
        &self,
        symbol: &str,
        name: &str,
        chain: Chain,
        decimals: u32,
    ) -> Result<networth_domain::CryptoAsset, MigrationError> {
        if let Some(asset) = self.crypto.find_asset_by_symbol(symbol).await? {
            return Ok(asset);
        }
        Ok(self
            .crypto
            .create_asset(&NewCryptoAsset {
                symbol: symbol.to_string(),
                name: name.to_string(),
                chain,
                contract_address: None,
                decimals,
                currency: Currency::usd(),
                price_provider_id: None,
            })
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    async fn ensure_wallet(
        &self,
        asset_symbol: &str,
        asset_name: &str,
        chain: Chain,
        decimals: u32,
        label: &str,
        kind: CryptoWalletKind,
        address: Option<String>,
        ownership_scope: OwnershipScope,
        wallets: &mut Vec<networth_domain::CryptoWallet>,
        new_wallets: &mut HashSet<Uuid>,
        migration_id: Uuid,
        source: &str,
    ) -> Result<networth_domain::CryptoWallet, MigrationError> {
        let asset = self
            .ensure_crypto_asset(asset_symbol, asset_name, chain, decimals)
            .await?;
        if let Some(wallet) = wallets.iter().find(|wallet| {
            wallet.asset.id == asset.id
                && wallet.kind == kind
                && if kind == CryptoWalletKind::Manual {
                    wallet.label.eq_ignore_ascii_case(label)
                } else {
                    wallet.address == address
                }
        }) {
            return Ok(wallet.clone());
        }
        let wallet = self
            .crypto
            .create_wallet(&NewCryptoWallet {
                asset_id: asset.id,
                kind,
                chain,
                address,
                label: label.to_string(),
                ownership_scope,
                owner_user_id: None,
            })
            .await?;
        self.migrations
            .record_provenance(migration_id, "crypto_wallet", wallet.id, source)
            .await?;
        new_wallets.insert(wallet.id);
        wallets.push(wallet.clone());
        Ok(wallet)
    }
}

/// Parse a decimal string for callers that need early validation.
pub fn parse_decimal_text(value: &str) -> Result<BigDecimal, String> {
    BigDecimal::from_str(value).map_err(|error| error.to_string())
}
