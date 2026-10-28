//! High-precision asset quantities (for example cryptocurrency amounts).

use std::fmt;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Decimal places the type must support without rounding.
///
/// Cryptocurrency commonly needs 18 decimals (for example 1 wei of Ether);
/// fiat needs at least 2. The type itself is arbitrary precision and this
/// constant exists to make the requirement explicit and testable.
pub const QUANTITY_DECIMALS: u32 = 18;

/// Errors raised when constructing a [`Quantity`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QuantityError {
    /// A quantity may not be negative.
    #[error("quantity must not be negative: {0}")]
    Negative(String),
    /// The text could not be parsed as a decimal.
    #[error("invalid quantity: {0}")]
    Invalid(String),
}

/// A non-negative, arbitrary-precision asset quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantity(BigDecimal);

impl Quantity {
    /// Build a quantity, rejecting negative values.
    pub fn new(value: BigDecimal) -> Result<Self, QuantityError> {
        if value < 0 {
            Err(QuantityError::Negative(value.to_string()))
        } else {
            Ok(Self(value))
        }
    }

    /// Parse a quantity from decimal text.
    pub fn parse(value: &str) -> Result<Self, QuantityError> {
        let parsed = BigDecimal::from_str(value.trim())
            .map_err(|_| QuantityError::Invalid(value.to_string()))?;
        Self::new(parsed)
    }

    /// The exact decimal quantity.
    pub fn as_decimal(&self) -> &BigDecimal {
        &self.0
    }

    /// Add two quantities exactly.
    pub fn checked_add(&self, other: &Quantity) -> Quantity {
        Quantity(&self.0 + &other.0)
    }

    /// Multiply by a non-negative decimal factor (for example a unit price).
    pub fn checked_mul(&self, factor: &BigDecimal) -> Quantity {
        Quantity(&self.0 * factor)
    }

    /// Zero quantity.
    pub fn zero() -> Self {
        Self(BigDecimal::from(0))
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::decimal::to_plain_string(&self.0))
    }
}

/// Serialised as a decimal **string** so precision survives JSON.
impl Serialize for Quantity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&crate::decimal::to_plain_string(&self.0))
    }
}

impl<'de> Deserialize<'de> for Quantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Quantity::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_eighteen_decimals() {
        let dust = "0.000000000000000001";
        let quantity = Quantity::parse(dust).unwrap();
        assert_eq!(quantity.to_string(), dust);
    }

    #[test]
    fn eighteen_decimals_survive_json_round_trip() {
        let dust = Quantity::parse("0.000000000000000001").unwrap();
        let json = serde_json::to_string(&dust).unwrap();
        assert_eq!(json, "\"0.000000000000000001\"");
        let back: Quantity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.to_string(), "0.000000000000000001");
    }

    #[test]
    fn rejects_negative_quantities() {
        assert!(matches!(
            Quantity::parse("-1"),
            Err(QuantityError::Negative(_))
        ));
    }

    #[test]
    fn adds_exactly() {
        let a = Quantity::parse("0.000000000000000001").unwrap();
        let b = Quantity::parse("0.000000000000000002").unwrap();
        assert_eq!(a.checked_add(&b).to_string(), "0.000000000000000003");
    }
}
