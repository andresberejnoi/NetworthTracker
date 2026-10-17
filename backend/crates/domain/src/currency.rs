//! Currency codes.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A three-letter, uppercase currency code such as `USD`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Currency(String);

/// Errors raised when constructing a [`Currency`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CurrencyError {
    /// The code was not three ASCII letters.
    #[error("currency code must be exactly 3 letters, got {0:?}")]
    Invalid(String),
}

impl Currency {
    /// Parse and normalise a currency code (case-insensitive, trimmed).
    pub fn new(code: impl Into<String>) -> Result<Self, CurrencyError> {
        let upper = code.into().trim().to_ascii_uppercase();
        if upper.len() == 3 && upper.chars().all(|c| c.is_ascii_alphabetic()) {
            Ok(Self(upper))
        } else {
            Err(CurrencyError::Invalid(upper))
        }
    }

    /// The US dollar, the v1 reporting currency.
    pub fn usd() -> Self {
        Self("USD".to_string())
    }

    /// The code as an uppercase string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Currency {
    type Err = CurrencyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for Currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_normalises() {
        assert_eq!(Currency::new("usd").unwrap().as_str(), "USD");
        assert_eq!(Currency::new(" eur ").unwrap().as_str(), "EUR");
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(Currency::new("US").is_err());
        assert!(Currency::new("USDD").is_err());
        assert!(Currency::new("U$D").is_err());
    }

    #[test]
    fn serde_round_trip() {
        let json = serde_json::to_string(&Currency::usd()).unwrap();
        assert_eq!(json, "\"USD\"");
        let back: Currency = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Currency::usd());
    }
}
