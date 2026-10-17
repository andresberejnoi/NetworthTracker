# Money and quantities

NetworthTracker never represents money with binary floating point. `0.1 + 0.2`
must be `0.3`, not `0.30000000000000004`, because rounding errors compound across
a portfolio and make net worth subtly wrong.

Two domain types enforce this:

- `Money` — an exact decimal **plus** a currency code.
- `Quantity` — an exact, non-negative decimal for things like crypto amounts.

## Money is exact

```rust
use networth_domain::Money;

let balance = Money::parse("1000.00", "usd").unwrap();
let contribution = Money::parse("250.555", "USD").unwrap();
assert_eq!((balance + contribution).to_string(), "1250.555 USD");
```

The classic floating-point trap does not apply:

```rust
use networth_domain::Money;

let sum = Money::parse("0.1", "USD").unwrap() + Money::parse("0.2", "USD").unwrap();
assert_eq!(sum.to_string(), "0.30 USD");
```

Amounts keep their full decimal precision — large balances and many decimal
places are preserved without rounding:

```rust
use networth_domain::Money;

let tiny = Money::parse("0.000000000000000001", "USD").unwrap();
assert_eq!(tiny.to_string(), "0.000000000000000001 USD");
```

## Currencies never mix silently

```rust
use networth_domain::Money;

let usd = Money::parse("1.00", "USD").unwrap();
let eur = Money::parse("1.00", "EUR").unwrap();
assert!(usd.checked_add(&eur).is_err());
```

## Amounts travel as JSON strings

An amount is serialised as a JSON **string**, so no consumer is tempted to parse
it as a float:

```rust
use networth_domain::Money;

let json = serde_json::to_string(&Money::parse("42.50", "USD").unwrap()).unwrap();
assert_eq!(json, r#"{"amount":"42.50","currency":"USD"}"#);
```

## Quantities support at least 18 decimals

Cryptocurrency needs far more precision than fiat. `Quantity` guarantees at
least [`QUANTITY_DECIMALS`](crate::quantity::QUANTITY_DECIMALS) (18) decimal
places and rejects negative values:

```rust
use networth_domain::Quantity;

let dust = Quantity::parse("0.000000000000000001").unwrap();
assert_eq!(dust.to_string(), "0.000000000000000001");
assert!(Quantity::parse("-1").is_err());
```

## Rules

1. Never use `f32`/`f64` for money or quantities.
2. Persist amounts in a decimal column (`NUMERIC`), not `REAL`.
3. Tag every amount with a currency; v1 reports in `USD` but the model does not
   assume two decimal places or a single currency.
