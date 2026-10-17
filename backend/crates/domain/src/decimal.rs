//! Plain-decimal formatting.
//!
//! `BigDecimal`'s `Display` renders very small or very large values in
//! scientific notation (for example `1E-18`). Money and quantities must always
//! render as plain decimal text, so serialisation and display go through
//! [`to_plain_string`].

use bigdecimal::BigDecimal;

/// Render a decimal as plain decimal text, never scientific notation.
pub(crate) fn to_plain_string(value: &BigDecimal) -> String {
    let (mantissa, scale) = value.as_bigint_and_exponent();
    let raw = mantissa.to_string();
    let negative = raw.starts_with('-');
    let digits = raw.trim_start_matches('-');
    let scale = scale.max(0) as usize;

    let mut out = String::new();
    if negative {
        out.push('-');
    }

    if scale == 0 {
        out.push_str(digits);
        return out;
    }

    if digits.len() > scale {
        let split = digits.len() - scale;
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
    } else {
        out.push_str("0.");
        for _ in 0..(scale - digits.len()) {
            out.push('0');
        }
        out.push_str(digits);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn renders_plain_decimal_text() {
        for (input, expected) in [
            ("0.000000000000000001", "0.000000000000000001"),
            ("0.1", "0.1"),
            ("42.50", "42.50"),
            (
                "123456789012345678901234567890.123456789",
                "123456789012345678901234567890.123456789",
            ),
            ("-5", "-5"),
            ("0", "0"),
        ] {
            let value = BigDecimal::from_str(input).unwrap();
            assert_eq!(to_plain_string(&value), expected, "input {input}");
        }
    }
}
