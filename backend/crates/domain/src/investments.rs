//! Securities, holdings, tax lots, and price quotes.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::currency::Currency;
use crate::money::Money;
use crate::quantity::Quantity;

/// The asset class of an individual security.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAssetClass {
    /// Stocks and equity funds.
    Equity,
    /// Bonds and fixed-income funds.
    FixedIncome,
    /// Real-estate securities and REITs.
    RealEstate,
    /// Commodities and commodity funds.
    Commodity,
    /// Cash equivalents.
    Cash,
    /// Anything else.
    Other,
}

impl SecurityAssetClass {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityAssetClass::Equity => "equity",
            SecurityAssetClass::FixedIncome => "fixed_income",
            SecurityAssetClass::RealEstate => "real_estate",
            SecurityAssetClass::Commodity => "commodity",
            SecurityAssetClass::Cash => "cash",
            SecurityAssetClass::Other => "other",
        }
    }
}

impl fmt::Display for SecurityAssetClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`SecurityAssetClass`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid asset class: {0}")]
pub struct SecurityAssetClassParseError(pub String);

impl FromStr for SecurityAssetClass {
    type Err = SecurityAssetClassParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "equity" => Ok(SecurityAssetClass::Equity),
            "fixed_income" => Ok(SecurityAssetClass::FixedIncome),
            "real_estate" => Ok(SecurityAssetClass::RealEstate),
            "commodity" => Ok(SecurityAssetClass::Commodity),
            "cash" => Ok(SecurityAssetClass::Cash),
            "other" => Ok(SecurityAssetClass::Other),
            other => Err(SecurityAssetClassParseError(other.to_string())),
        }
    }
}

/// A GICS-style sector, used for equity allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    /// Energy.
    Energy,
    /// Materials.
    Materials,
    /// Industrials.
    Industrials,
    /// Consumer discretionary.
    ConsumerDiscretionary,
    /// Consumer staples.
    ConsumerStaples,
    /// Health care.
    HealthCare,
    /// Financials.
    Financials,
    /// Information technology.
    InformationTechnology,
    /// Communication services.
    CommunicationServices,
    /// Utilities.
    Utilities,
    /// Real estate.
    RealEstate,
    /// Unclassified or other.
    Other,
}

impl Sector {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            Sector::Energy => "energy",
            Sector::Materials => "materials",
            Sector::Industrials => "industrials",
            Sector::ConsumerDiscretionary => "consumer_discretionary",
            Sector::ConsumerStaples => "consumer_staples",
            Sector::HealthCare => "health_care",
            Sector::Financials => "financials",
            Sector::InformationTechnology => "information_technology",
            Sector::CommunicationServices => "communication_services",
            Sector::Utilities => "utilities",
            Sector::RealEstate => "real_estate",
            Sector::Other => "other",
        }
    }
}

impl fmt::Display for Sector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`Sector`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid sector: {0}")]
pub struct SectorParseError(pub String);

impl FromStr for Sector {
    type Err = SectorParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "energy" => Ok(Sector::Energy),
            "materials" => Ok(Sector::Materials),
            "industrials" => Ok(Sector::Industrials),
            "consumer_discretionary" => Ok(Sector::ConsumerDiscretionary),
            "consumer_staples" => Ok(Sector::ConsumerStaples),
            "health_care" => Ok(Sector::HealthCare),
            "financials" => Ok(Sector::Financials),
            "information_technology" => Ok(Sector::InformationTechnology),
            "communication_services" => Ok(Sector::CommunicationServices),
            "utilities" => Ok(Sector::Utilities),
            "real_estate" => Ok(Sector::RealEstate),
            "other" => Ok(Sector::Other),
            other => Err(SectorParseError(other.to_string())),
        }
    }
}

/// A tradeable security (stock, ETF, fund, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Security {
    /// Surrogate identifier.
    pub id: Uuid,
    /// Uppercase ticker symbol.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Asset class.
    pub asset_class: SecurityAssetClass,
    /// Sector, for equities.
    pub sector: Option<Sector>,
    /// Quote currency.
    pub currency: Currency,
}

/// The fields required to create a security.
#[derive(Debug, Clone)]
pub struct NewSecurity {
    /// Ticker symbol (normalised to uppercase).
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Asset class.
    pub asset_class: SecurityAssetClass,
    /// Sector, for equities.
    pub sector: Option<Sector>,
    /// Quote currency.
    pub currency: Currency,
}

/// Where a lot came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LotSource {
    /// A normal purchase.
    Purchase,
    /// Income received in kind (staking, mining, airdrop).
    Income,
    /// Moved from another wallet or account.
    Transfer,
    /// Imported from a broker/exchange file.
    Import,
    /// A correcting adjustment.
    Adjustment,
}

impl LotSource {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            LotSource::Purchase => "purchase",
            LotSource::Income => "income",
            LotSource::Transfer => "transfer",
            LotSource::Import => "import",
            LotSource::Adjustment => "adjustment",
        }
    }
}

impl fmt::Display for LotSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`LotSource`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid lot source: {0}")]
pub struct LotSourceParseError(pub String);

impl FromStr for LotSource {
    type Err = LotSourceParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "purchase" => Ok(LotSource::Purchase),
            "income" => Ok(LotSource::Income),
            "transfer" => Ok(LotSource::Transfer),
            "import" => Ok(LotSource::Import),
            "adjustment" => Ok(LotSource::Adjustment),
            other => Err(LotSourceParseError(other.to_string())),
        }
    }
}

/// Whether a lot's cost basis is known or estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasisQuality {
    /// Actual purchase records.
    Actual,
    /// Approximate/backfilled basis.
    Estimated,
}

impl BasisQuality {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            BasisQuality::Actual => "actual",
            BasisQuality::Estimated => "estimated",
        }
    }
}

impl fmt::Display for BasisQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`BasisQuality`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid basis quality: {0}")]
pub struct BasisQualityParseError(pub String);

impl FromStr for BasisQuality {
    type Err = BasisQualityParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "actual" => Ok(BasisQuality::Actual),
            "estimated" => Ok(BasisQuality::Estimated),
            other => Err(BasisQualityParseError(other.to_string())),
        }
    }
}

/// The lot-consumption method used by a sale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleMethod {
    /// First in, first out.
    Fifo,
}

impl SaleMethod {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            SaleMethod::Fifo => "fifo",
        }
    }
}

impl fmt::Display for SaleMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`SaleMethod`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid sale method: {0}")]
pub struct SaleMethodParseError(pub String);

impl FromStr for SaleMethod {
    type Err = SaleMethodParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fifo" => Ok(SaleMethod::Fifo),
            other => Err(SaleMethodParseError(other.to_string())),
        }
    }
}

/// How the caller defines a long-term holding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongTermRule {
    /// Long-term when the disposal is after the acquisition anniversary.
    MoreThanYears(u16),
    /// Long-term when the disposal is more than this many days after acquisition.
    MoreThanDays(u32),
}

impl Default for LongTermRule {
    fn default() -> Self {
        Self::MoreThanYears(1)
    }
}

impl LongTermRule {
    /// Whether a holding is long-term under this rule.
    pub fn is_long_term(self, acquired_on: Date, disposed_on: Date) -> bool {
        match self {
            LongTermRule::MoreThanYears(years) => disposed_on > add_years(acquired_on, years),
            LongTermRule::MoreThanDays(days) => {
                (disposed_on - acquired_on).whole_days() > i64::from(days)
            }
        }
    }
}

/// A derived short/long holding-period label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldingTerm {
    /// Held for the short-term period.
    Short,
    /// Held beyond the long-term threshold.
    Long,
}

impl HoldingTerm {
    /// Stable identifier for the API and reports.
    pub fn as_str(self) -> &'static str {
        match self {
            HoldingTerm::Short => "short",
            HoldingTerm::Long => "long",
        }
    }
}

impl fmt::Display for HoldingTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A tax lot inside a holding.
///
/// The original quantity/basis are immutable facts; the remaining values are
/// reduced by disposals and transfers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lot {
    /// Surrogate identifier.
    pub id: Uuid,
    /// Insertion sequence, used to break same-day FIFO ties deterministically.
    pub sequence: i64,
    /// Acquisition date.
    pub acquired_on: Date,
    /// Quantity originally acquired.
    pub original_quantity: Quantity,
    /// Quantity still held.
    pub remaining_quantity: Quantity,
    /// Total original cost basis.
    pub original_cost_basis: Money,
    /// Cost basis still held.
    pub remaining_cost_basis: Money,
    /// Fees paid on acquisition.
    pub fees: Money,
    /// Where the lot came from.
    pub source: LotSource,
    /// Whether the basis is actual or estimated.
    pub basis_quality: BasisQuality,
    /// Optional note explaining an estimated basis or correction.
    pub basis_note: Option<String>,
}

impl Lot {
    /// The remaining quantity.
    pub fn quantity(&self) -> Quantity {
        self.remaining_quantity.clone()
    }

    /// The remaining cost basis.
    pub fn cost_basis(&self) -> Money {
        self.remaining_cost_basis.clone()
    }
}

/// The fields required to record a lot.
#[derive(Debug, Clone)]
pub struct NewLot {
    /// Acquisition date.
    pub acquired_on: Date,
    /// Quantity acquired.
    pub quantity: Quantity,
    /// Total cost of the lot.
    pub cost_basis: Money,
    /// Fees paid on acquisition.
    pub fees: Option<Money>,
    /// Where the lot came from.
    pub source: LotSource,
    /// Whether the basis is actual or estimated.
    pub basis_quality: BasisQuality,
    /// Optional note.
    pub basis_note: Option<String>,
}

/// A security position inside an account, made of one or more lots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    /// Surrogate identifier.
    pub id: Uuid,
    /// The account that owns the position.
    pub account_id: Uuid,
    /// The security.
    pub security: Security,
    /// The tax lots.
    pub lots: Vec<Lot>,
}

impl Holding {
    /// Total remaining quantity across all lots.
    pub fn quantity(&self) -> Quantity {
        self.lots.iter().fold(Quantity::zero(), |total, lot| {
            total.checked_add(&lot.remaining_quantity)
        })
    }

    /// Total remaining cost basis across all lots.
    pub fn cost_basis(&self) -> Money {
        let mut total = Money::zero(self.security.currency.clone());
        for lot in &self.lots {
            total = total
                .checked_add(&lot.remaining_cost_basis)
                .expect("lot currencies must match the security");
        }
        total
    }

    /// Market value at the given unit price.
    pub fn market_value(&self, price: &Money) -> Money {
        Money::new(
            self.quantity().as_decimal() * price.amount(),
            price.currency().clone(),
        )
    }

    /// Unrealized gain or loss at the given unit price.
    pub fn unrealized_gain(&self, price: &Money) -> Money {
        self.market_value(price)
            .checked_sub(&self.cost_basis())
            .expect("price and cost basis currencies must match")
    }
}

/// A sale request.
#[derive(Debug, Clone)]
pub struct NewSale {
    /// Disposal date.
    pub disposed_on: Date,
    /// Quantity sold.
    pub quantity: Quantity,
    /// Gross proceeds before fees.
    pub gross_proceeds: Money,
    /// Selling fees.
    pub fees: Money,
    /// Currency of the proceeds and fees.
    pub currency: Currency,
    /// Lot-consumption method.
    pub method: SaleMethod,
}

/// One lot consumed by a sale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disposal {
    /// Surrogate identifier.
    pub id: Uuid,
    /// The consumed lot.
    pub lot_id: Uuid,
    /// The lot's acquisition date.
    pub acquired_on: Date,
    /// Quantity consumed.
    pub quantity: Quantity,
    /// Gross proceeds allocated to this lot.
    pub gross_proceeds: Money,
    /// Fees allocated to this lot.
    pub fees: Money,
    /// Cost basis allocated to this lot.
    pub cost_basis: Money,
    /// Whether the lot's basis was actual or estimated.
    pub basis_quality: BasisQuality,
}

impl Disposal {
    /// Net proceeds after allocated fees.
    pub fn net_proceeds(&self) -> Money {
        self.gross_proceeds
            .checked_sub(&self.fees)
            .expect("proceeds and fees share a currency")
    }

    /// Realized gain/loss for this lot.
    pub fn realized_gain(&self) -> Money {
        self.net_proceeds()
            .checked_sub(&self.cost_basis)
            .expect("proceeds and cost basis share a currency")
    }

    /// Days between acquisition and disposal.
    pub fn holding_period_days(&self, disposed_on: Date) -> i64 {
        (disposed_on - self.acquired_on).whole_days()
    }

    /// Derived short/long-term label.
    pub fn term(&self, disposed_on: Date, rule: LongTermRule) -> HoldingTerm {
        if rule.is_long_term(self.acquired_on, disposed_on) {
            HoldingTerm::Long
        } else {
            HoldingTerm::Short
        }
    }
}

/// A sale and the lots it consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sale {
    /// Surrogate identifier.
    pub id: Uuid,
    /// The account that owns the position.
    pub account_id: Uuid,
    /// The position sold.
    pub holding_id: Uuid,
    /// The security.
    pub security: Security,
    /// Disposal date.
    pub disposed_on: Date,
    /// Quantity sold.
    pub quantity: Quantity,
    /// Gross proceeds before fees.
    pub gross_proceeds: Money,
    /// Selling fees.
    pub fees: Money,
    /// Currency.
    pub currency: Currency,
    /// Lot-consumption method.
    pub method: SaleMethod,
    /// Per-lot disposal rows.
    pub disposals: Vec<Disposal>,
}

impl Sale {
    /// Total cost basis consumed.
    pub fn cost_basis(&self) -> Money {
        let mut total = Money::zero(self.currency.clone());
        for disposal in &self.disposals {
            total = total
                .checked_add(&disposal.cost_basis)
                .expect("disposals share a currency");
        }
        total
    }

    /// Net proceeds after fees.
    pub fn net_proceeds(&self) -> Money {
        self.gross_proceeds
            .checked_sub(&self.fees)
            .expect("proceeds and fees share a currency")
    }

    /// Realized gain/loss.
    pub fn realized_gain(&self) -> Money {
        self.net_proceeds()
            .checked_sub(&self.cost_basis())
            .expect("proceeds and cost basis share a currency")
    }
}

/// A FIFO allocation for one lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalAllocation {
    /// The lot consumed.
    pub lot_id: Uuid,
    /// The lot's acquisition date.
    pub acquired_on: Date,
    /// Quantity consumed.
    pub quantity: Quantity,
    /// Gross proceeds allocated.
    pub gross_proceeds: Money,
    /// Fees allocated.
    pub fees: Money,
    /// Cost basis allocated.
    pub cost_basis: Money,
    /// Whether the lot's basis was actual or estimated.
    pub basis_quality: BasisQuality,
}

/// Errors raised by lot allocation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LotError {
    /// The sale quantity must be positive.
    #[error("sale quantity must be positive")]
    InvalidQuantity,
    /// There are not enough remaining units.
    #[error("cannot sell {requested}: only {available} available")]
    InsufficientQuantity {
        /// Requested quantity.
        requested: String,
        /// Available quantity.
        available: String,
    },
    /// Currencies do not match.
    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(String, String),
}

/// Allocate a sale across lots using FIFO.
///
/// The caller persists the allocations and reduces each lot's remaining
/// quantity/basis by the allocated amounts.
pub fn allocate_fifo(lots: &[Lot], sale: &NewSale) -> Result<Vec<DisposalAllocation>, LotError> {
    if sale.quantity.is_zero() {
        return Err(LotError::InvalidQuantity);
    }
    if sale.gross_proceeds.currency() != &sale.currency || sale.fees.currency() != &sale.currency {
        return Err(LotError::CurrencyMismatch(
            sale.currency.to_string(),
            sale.gross_proceeds.currency().to_string(),
        ));
    }

    let available = lots.iter().fold(Quantity::zero(), |total, lot| {
        total.checked_add(&lot.remaining_quantity)
    });
    if available < sale.quantity {
        return Err(LotError::InsufficientQuantity {
            requested: sale.quantity.to_string(),
            available: available.to_string(),
        });
    }

    let mut ordered: Vec<&Lot> = lots
        .iter()
        .filter(|lot| !lot.remaining_quantity.is_zero())
        .collect();
    ordered.sort_by_key(|lot| (lot.acquired_on, lot.sequence, lot.id));

    let mut remaining = sale.quantity.clone();
    let mut gross_left = sale.gross_proceeds.amount().clone();
    let mut fees_left = sale.fees.amount().clone();
    let mut allocations = Vec::new();

    for lot in ordered {
        if remaining.is_zero() {
            break;
        }
        if lot.remaining_cost_basis.currency() != &sale.currency {
            return Err(LotError::CurrencyMismatch(
                sale.currency.to_string(),
                lot.remaining_cost_basis.currency().to_string(),
            ));
        }

        let take = remaining.at_most(&lot.remaining_quantity);
        let is_last = take == remaining;
        let gross = if is_last {
            gross_left.clone()
        } else {
            allocate(
                sale.gross_proceeds.amount(),
                take.as_decimal(),
                sale.quantity.as_decimal(),
            )
        };
        let fees = if is_last {
            fees_left.clone()
        } else {
            allocate(
                sale.fees.amount(),
                take.as_decimal(),
                sale.quantity.as_decimal(),
            )
        };
        let cost_basis = if take == lot.remaining_quantity {
            lot.remaining_cost_basis.amount().clone()
        } else {
            allocate(
                lot.remaining_cost_basis.amount(),
                take.as_decimal(),
                lot.remaining_quantity.as_decimal(),
            )
        };

        allocations.push(DisposalAllocation {
            lot_id: lot.id,
            acquired_on: lot.acquired_on,
            quantity: take.clone(),
            gross_proceeds: Money::new(gross.clone(), sale.currency.clone()),
            fees: Money::new(fees.clone(), sale.currency.clone()),
            cost_basis: Money::new(cost_basis, sale.currency.clone()),
            basis_quality: lot.basis_quality,
        });

        remaining = remaining
            .checked_sub(&take)
            .expect("take never exceeds remaining");
        gross_left -= gross;
        fees_left -= fees;
    }

    Ok(allocations)
}

fn allocate(
    total: &bigdecimal::BigDecimal,
    part: &bigdecimal::BigDecimal,
    whole: &bigdecimal::BigDecimal,
) -> bigdecimal::BigDecimal {
    if part == whole {
        total.clone()
    } else {
        (total * part) / whole
    }
}

fn add_years(date: Date, years: u16) -> Date {
    let year = date.year() + i32::from(years);
    Date::from_calendar_date(year, date.month(), date.day())
        .or_else(|_| Date::from_calendar_date(year, date.month(), date.day() - 1))
        .unwrap_or(date)
}

/// Where a price came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// Retrieved from a configured provider.
    Provider,
    /// Entered by the user.
    Manual,
}

impl PriceSource {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            PriceSource::Provider => "provider",
            PriceSource::Manual => "manual",
        }
    }
}

impl fmt::Display for PriceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`PriceSource`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid price source: {0}")]
pub struct PriceSourceParseError(pub String);

impl FromStr for PriceSource {
    type Err = PriceSourceParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "provider" => Ok(PriceSource::Provider),
            "manual" => Ok(PriceSource::Manual),
            other => Err(PriceSourceParseError(other.to_string())),
        }
    }
}

/// A unit price with its provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceQuote {
    /// Ticker symbol.
    pub symbol: String,
    /// Unit price.
    pub price: Money,
    /// When the price was observed.
    pub as_of: OffsetDateTime,
    /// Whether the price is stale (cached because no provider is available).
    pub stale: bool,
    /// Where the price came from.
    pub source: PriceSource,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;
    use std::str::FromStr;
    use time::macros::date;

    fn security() -> Security {
        Security {
            id: Uuid::new_v4(),
            symbol: "VTI".to_string(),
            name: "Total Market ETF".to_string(),
            asset_class: SecurityAssetClass::Equity,
            sector: Some(Sector::Other),
            currency: Currency::usd(),
        }
    }

    fn lot(quantity: &str, cost: &str, day: i64) -> Lot {
        let quantity = Quantity::parse(quantity).unwrap();
        let cost = Money::parse(cost, "USD").unwrap();
        Lot {
            id: Uuid::new_v4(),
            sequence: 0,
            acquired_on: date!(2026 - 01 - 01) + time::Duration::days(day),
            original_quantity: quantity.clone(),
            remaining_quantity: quantity,
            original_cost_basis: cost.clone(),
            remaining_cost_basis: cost,
            fees: Money::zero(Currency::usd()),
            source: LotSource::Purchase,
            basis_quality: BasisQuality::Actual,
            basis_note: None,
        }
    }

    fn sale(quantity: &str, proceeds: &str, fees: &str, day: i64) -> NewSale {
        NewSale {
            disposed_on: date!(2026 - 01 - 01) + time::Duration::days(day),
            quantity: Quantity::parse(quantity).unwrap(),
            gross_proceeds: Money::parse(proceeds, "USD").unwrap(),
            fees: Money::parse(fees, "USD").unwrap(),
            currency: Currency::usd(),
            method: SaleMethod::Fifo,
        }
    }

    #[test]
    fn aggregates_multiple_lots() {
        let holding = Holding {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            security: security(),
            lots: vec![lot("10", "1000.00", 0), lot("5.5", "600.00", 30)],
        };

        assert_eq!(holding.quantity().to_string(), "15.5");
        assert_eq!(holding.cost_basis().amount_string(), "1600.00");
    }

    #[test]
    fn values_a_holding_and_reports_gain() {
        let holding = Holding {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            security: security(),
            lots: vec![lot("10", "1000.00", 0)],
        };
        let price = Money::parse("150.00", "USD").unwrap();

        assert_eq!(holding.market_value(&price).amount_string(), "1500.00");
        assert_eq!(holding.unrealized_gain(&price).amount_string(), "500.00");
    }

    #[test]
    fn reports_a_loss() {
        let holding = Holding {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            security: security(),
            lots: vec![lot("10", "1000.00", 0)],
        };
        let price = Money::parse("80.00", "USD").unwrap();

        assert_eq!(holding.unrealized_gain(&price).amount_string(), "-200.00");
    }

    #[test]
    fn parses_classification_and_sector() {
        assert_eq!(
            SecurityAssetClass::from_str("fixed_income").unwrap(),
            SecurityAssetClass::FixedIncome
        );
        assert_eq!(
            Sector::from_str("information_technology").unwrap(),
            Sector::InformationTechnology
        );
        assert!(SecurityAssetClass::from_str("nope").is_err());
        assert!(Sector::from_str("nope").is_err());
    }
    #[test]
    fn fifo_consumes_oldest_lots_first() {
        let lots = vec![lot("10", "1000.00", 0), lot("5", "600.00", 31)];
        let allocations = allocate_fifo(&lots, &sale("12", "1800.00", "12.00", 60)).unwrap();

        assert_eq!(allocations.len(), 2);
        assert_eq!(allocations[0].quantity.to_string(), "10");
        assert_eq!(allocations[0].cost_basis.amount_string(), "1000.00");
        assert_eq!(allocations[1].quantity.to_string(), "2");
        assert_eq!(allocations[1].cost_basis.amount_string(), "240.00");
        assert_eq!(allocations[1].gross_proceeds.amount_string(), "300.00");
    }

    #[test]
    fn rejects_selling_more_than_available() {
        let lots = vec![lot("1", "100.00", 0)];
        let error = allocate_fifo(&lots, &sale("2", "300.00", "0", 10)).unwrap_err();
        assert!(matches!(error, LotError::InsufficientQuantity { .. }));
    }

    #[test]
    fn long_term_uses_calendar_anniversary() {
        let acquired = date!(2023 - 03 - 01);
        assert!(!LongTermRule::MoreThanYears(1).is_long_term(acquired, date!(2024 - 03 - 01)));
        assert!(LongTermRule::MoreThanYears(1).is_long_term(acquired, date!(2024 - 03 - 02)));

        let acquired = date!(2024 - 06 - 01);
        assert!(!LongTermRule::MoreThanYears(1).is_long_term(acquired, date!(2025 - 06 - 01)));
        assert!(LongTermRule::MoreThanYears(1).is_long_term(acquired, date!(2025 - 06 - 02)));
    }

    #[test]
    fn days_rule_is_exact() {
        let acquired = date!(2026 - 01 - 01);
        assert!(!LongTermRule::MoreThanDays(10).is_long_term(acquired, date!(2026 - 01 - 11)));
        assert!(LongTermRule::MoreThanDays(10).is_long_term(acquired, date!(2026 - 01 - 12)));
    }

    #[test]
    fn estimated_basis_is_metadata_only() {
        let mut lots = vec![lot("1", "100.00", 0)];
        lots[0].basis_quality = BasisQuality::Estimated;
        let allocations = allocate_fifo(&lots, &sale("1", "150.00", "0", 400)).unwrap();
        assert_eq!(allocations[0].basis_quality, BasisQuality::Estimated);
        assert_eq!(allocations[0].cost_basis.amount_string(), "100.00");
    }
}
