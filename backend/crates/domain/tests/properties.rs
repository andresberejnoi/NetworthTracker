//! Property-based tests for exact money arithmetic.

use bigdecimal::BigDecimal;
use networth_domain::{Currency, Money, Quantity};
use proptest::prelude::*;

/// Generate an amount from an integer and a scale so values are exact decimals
/// rather than floats.
fn amount() -> impl Strategy<Value = BigDecimal> {
    (-1_000_000_000i64..1_000_000_000, 0u32..18)
        .prop_map(|(value, scale)| BigDecimal::new(value.into(), scale as i64))
}

proptest! {
    /// Adding then subtracting the same amount returns the original value.
    #[test]
    fn add_then_subtract_is_identity(a in amount(), b in amount()) {
        let currency = Currency::usd();
        let a_money = Money::new(a.clone(), currency.clone());
        let b_money = Money::new(b.clone(), currency);

        let round_tripped = a_money
            .checked_add(&b_money)
            .unwrap()
            .checked_sub(&b_money)
            .unwrap();

        prop_assert_eq!(round_tripped.amount(), &a);
    }

    /// Addition is commutative.
    #[test]
    fn addition_is_commutative(a in amount(), b in amount()) {
        let currency = Currency::usd();
        let a_money = Money::new(a, currency.clone());
        let b_money = Money::new(b, currency);

        let left = a_money.checked_add(&b_money).unwrap();
        let right = b_money.checked_add(&a_money).unwrap();

        prop_assert_eq!(left.amount(), right.amount());
    }

    /// Money serialises to a JSON string and round-trips without precision loss.
    #[test]
    fn json_round_trip_is_lossless(a in amount()) {
        let money = Money::new(a.clone(), Currency::usd());
        let json = serde_json::to_string(&money).unwrap();
        let back: Money = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back.amount(), &a);
    }

    /// Quantities hold at least 18 decimals without rounding.
    #[test]
    fn quantities_preserve_scale(value in 0i64..1_000_000, scale in 0u32..=18) {
        let decimal = BigDecimal::new(value.into(), scale as i64);
        let quantity = Quantity::new(decimal.clone()).unwrap();
        prop_assert_eq!(quantity.as_decimal(), &decimal);
    }
}
