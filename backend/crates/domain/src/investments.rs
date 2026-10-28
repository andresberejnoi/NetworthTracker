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

/// A tax lot inside a holding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lot {
    /// Surrogate identifier.
    pub id: Uuid,
    /// Acquisition date.
    pub acquired_on: Date,
    /// Quantity acquired.
    pub quantity: Quantity,
    /// Total cost of the lot, in the security's currency.
    pub cost_basis: Money,
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
    /// Total quantity across all lots.
    pub fn quantity(&self) -> Quantity {
        self.lots.iter().fold(Quantity::zero(), |total, lot| {
            total.checked_add(&lot.quantity)
        })
    }

    /// Total cost basis across all lots.
    pub fn cost_basis(&self) -> Money {
        let mut total = Money::zero(self.security.currency.clone());
        for lot in &self.lots {
            total = total
                .checked_add(&lot.cost_basis)
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

    fn lot(quantity: &str, cost: &str, day: u8) -> Lot {
        Lot {
            id: Uuid::new_v4(),
            acquired_on: date!(2026 - 01 - 01) + time::Duration::days(day as i64),
            quantity: Quantity::parse(quantity).unwrap(),
            cost_basis: Money::parse(cost, "USD").unwrap(),
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
}
