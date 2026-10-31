# Tax lots, sales, and realized gains

Investments and cryptocurrency use one append-only tax-lot ledger. A lot is a
purchase fact: acquisition date, quantity, total cost basis, fees, and whether
the basis is **actual** or **estimated**. Adding a lot never replaces existing
lots.

## Model

- **Lot** — original and remaining quantity/basis, acquisition date, fees,
  source (`purchase`, `income`, `transfer`, `import`, `adjustment`), and
  `basis_quality` (`actual` or `estimated`). `cost_basis` is the total tax basis
  and must include acquisition fees; the separate `fees` field is reporting
  metadata and is not added again.
- **Sale** — disposal date, quantity, gross proceeds, fees, currency, and the
  lot-consumption method. FIFO is the only method exposed in v1.
- **Disposal** — one row per lot consumed by a sale, with allocated proceeds,
  fees, cost basis, and realized gain.
- **Position** — the sum of remaining lots for an account/security pair.

## FIFO

A sale consumes lots in acquisition-date order (same-day ties are broken by insertion sequence). A sale
that spans lots creates one disposal per lot. Partial lots keep exact remaining
quantities and proportional cost basis.

Example:

```
Lot 1: 10 units, acquired 2026-01-01, basis 1000.00
Lot 2:  5 units, acquired 2026-02-01, basis  600.00
Sell 12 units on 2026-06-01 for 1800.00 gross, 12.00 fees

Disposal 1: 10 units, basis 1000.00, gross 1500.00, fees 10.00
Disposal 2:  2 units, basis  240.00, gross  300.00, fees  2.00
Remaining:   3 units, basis  360.00
Realized:    (1800.00 - 12.00) - 1240.00 = 548.00
```

## Holding period

The database stores dates only. The backend derives:

- `holding_period_days` — disposal date minus acquisition date.
- `term` — `short` or `long` under a caller-supplied rule.

The sale API accepts either:

```json
{ "long_term_years": 1 }
```

or

```json
{ "long_term_days": 365 }
```

The default is more than one calendar year. `long_term_years` uses the exact
calendar anniversary, so leap years are handled correctly. A region-to-rule
mapping belongs in config or an intermediary; the database does not store a
jurisdiction or a term column.

## Estimated basis

Historical imports often lack exact purchase records. Mark those lots
`basis_quality: "estimated"` and optionally add a note. Calculations use the
entered basis exactly like actual basis, but the UI and reports flag it so it
can be reviewed before filing.

## API

```bash
# append a purchase lot (adding another lot preserves existing lots)
curl -s -X POST localhost:8080/api/v1/accounts/<id>/holdings \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"VTI","lots":[{
        "acquired_on":"2026-01-01",
        "quantity":"10",
        "cost_basis":"1000.00",
        "fees":"1.00",
        "basis_quality":"actual"
      }]}'

# sell FIFO
curl -s -X POST localhost:8080/api/v1/accounts/<id>/sales \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"VTI","disposed_on":"2026-06-01",
       "quantity":"12","gross_proceeds":"1800.00","fees":"12.00",
       "long_term_years":1}'

# realized gains
curl -s 'localhost:8080/api/v1/accounts/<id>/sales?long_term_years=1' \
  -H "Authorization: Bearer $TOKEN"
```

## Current limits / next steps

- FIFO only. LIFO and specific-lot identification are deferred.
- FIFO is per account. Global asset-wide FIFO is deferred.
- Crypto wallets will use the same ledger; transfers preserve acquisition date
  and proportional basis.
- Wash sales, stock splits/mergers, crypto-to-crypto conversions, income lots,
  and fees paid in crypto are deferred.
- The JSON backup does not yet include investments; the full `pg_dump` backup
  does. This is tracked as a follow-up.
