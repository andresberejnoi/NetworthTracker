//! Exact monetary amounts.
//!
//! Money is stored as an arbitrary-precision decimal plus a currency, never as
//! a binary float. See `docs/money.md` for the rules and worked examples.

use std::fmt;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::currency::{Currency, CurrencyError};

/// Errors raised by money arithmetic.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MoneyError {
    /// Two amounts in different currencies cannot be combined.
    #[error("currency mismatch: cannot combine {left} and {right}")]
    CurrencyMismatch { left: String, right: String },
    /// An amount string could not be parsed as a decimal.
    #[error("invalid amount: {0}")]
    InvalidAmount(String),
    /// The currency code was invalid.
    #[error(transparent)]
    Currency(#[from] CurrencyError),
}

/// An exact monetary amount in a specific currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Money {
    amount: BigDecimal,
    currency: Currency,
}

impl Money {
    /// Build an amount from a decimal and a currency.
    pub fn new(amount: BigDecimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Parse an amount from decimal text and a currency code.
    pub fn parse(amount: &str, currency: &str) -> Result<Self, MoneyError> {
        let amount = BigDecimal::from_str(amount.trim())
            .map_err(|_| MoneyError::InvalidAmount(amount.to_string()))?;
        Ok(Self::new(amount, Currency::new(currency)?))
    }

    /// Zero in the given currency.
    pub fn zero(currency: Currency) -> Self {
        Self::new(BigDecimal::from(0), currency)
    }

    /// The exact decimal amount.
    pub fn amount(&self) -> &BigDecimal {
        &self.amount
    }

    /// The exact amount as plain decimal text (never scientific notation),
    /// with at least two decimal places.
    pub fn amount_string(&self) -> String {
        crate::decimal::to_canonical_money_string(&self.amount)
    }

    /// The currency.
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Add two amounts, failing on a currency mismatch.
    pub fn checked_add(&self, other: &Money) -> Result<Money, MoneyError> {
        self.ensure_same_currency(other)?;
        Ok(Money::new(
            &self.amount + &other.amount,
            self.currency.clone(),
        ))
    }

    /// Subtract `other` from `self`, failing on a currency mismatch.
    pub fn checked_sub(&self, other: &Money) -> Result<Money, MoneyError> {
        self.ensure_same_currency(other)?;
        Ok(Money::new(
            &self.amount - &other.amount,
            self.currency.clone(),
        ))
    }

    /// Negate the amount, keeping the currency.
    pub fn neg(&self) -> Money {
        Money::new(-&self.amount, self.currency.clone())
    }

    /// Whether the amount is below zero.
    pub fn is_negative(&self) -> bool {
        self.amount < 0
    }

    fn ensure_same_currency(&self, other: &Money) -> Result<(), MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch {
                left: self.currency.to_string(),
                right: other.currency.to_string(),
            });
        }
        Ok(())
    }
}

/// Adds two amounts. Panics on a currency mismatch; prefer [`Money::checked_add`].
impl std::ops::Add for Money {
    type Output = Money;

    fn add(self, rhs: Money) -> Money {
        self.checked_add(&rhs)
            .expect("currency mismatch when adding money")
    }
}

/// Subtracts two amounts. Panics on a currency mismatch; prefer [`Money::checked_sub`].
impl std::ops::Sub for Money {
    type Output = Money;

    fn sub(self, rhs: Money) -> Money {
        self.checked_sub(&rhs)
            .expect("currency mismatch when subtracting money")
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            crate::decimal::to_canonical_money_string(&self.amount),
            self.currency
        )
    }
}

/// Serialised as `{"amount": "<decimal string>", "currency": "<CODE>"}`.
///
/// The amount is always a JSON **string** so no consumer is tempted to parse it
/// as a float.
impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Money", 2)?;
        state.serialize_field(
            "amount",
            &crate::decimal::to_canonical_money_string(&self.amount),
        )?;
        state.serialize_field("currency", &self.currency)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Repr {
            amount: String,
            currency: String,
        }

        let repr = Repr::deserialize(deserializer)?;
        Money::parse(&repr.amount, &repr.currency).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd(amount: &str) -> Money {
        Money::parse(amount, "USD").unwrap()
    }

    #[test]
    fn add_and_subtract_are_exact() {
        let total = usd("10.25").checked_add(&usd("0.75")).unwrap();
        assert_eq!(total.to_string(), "11.00 USD");
        let back = total.checked_sub(&usd("0.75")).unwrap();
        assert_eq!(back.to_string(), "10.25 USD");
    }

    #[test]
    fn no_binary_float_drift() {
        // 0.1 + 0.2 is exactly 0.3, unlike f64.
        let sum = usd("0.1").checked_add(&usd("0.2")).unwrap();
        assert_eq!(sum.amount().to_string(), "0.3");
    }

    #[test]
    fn preserves_many_decimals_and_large_values() {
        let tiny = Money::parse("0.000000000000000001", "USD").unwrap();
        assert_eq!(tiny.to_string(), "0.000000000000000001 USD");

        let huge = "123456789012345678901234567890.123456789";
        let big = Money::parse(huge, "USD").unwrap();
        assert_eq!(big.to_string(), format!("{huge} USD"));
    }

    #[test]
    fn currency_mismatch_is_an_error() {
        let err = usd("1.00").checked_add(&Money::parse("1.00", "EUR").unwrap());
        assert!(matches!(err, Err(MoneyError::CurrencyMismatch { .. })));
    }

    #[test]
    fn serde_uses_strings_not_numbers() {
        let json = serde_json::to_string(&usd("42.50")).unwrap();
        assert_eq!(json, r#"{"amount":"42.50","currency":"USD"}"#);

        // The amount must be a JSON string, never a number.
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["amount"].is_string());

        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back.amount(), usd("42.50").amount());
    }

    #[test]
    fn accepts_numbers_as_input_but_never_native_floats() {
        // Deserialisation parses digits from a JSON string, so a value that
        // cannot be represented exactly as f64 still round-trips.
        let raw = r#"{"amount":"0.30000000000000004","currency":"USD"}"#;
        let money: Money = serde_json::from_str(raw).unwrap();
        assert_eq!(money.amount().to_string(), "0.30000000000000004");
    }

    #[test]
    fn neg_and_sign() {
        assert!(Money::parse("-5.00", "USD").unwrap().is_negative());
        assert!(!usd("0").is_negative());
        assert_eq!(usd("5").neg().to_string(), "-5.00 USD");
    }
}
