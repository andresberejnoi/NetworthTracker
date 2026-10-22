# Sample spreadsheets

These files use the versioned import format (`schema_version = 1`) and can be
imported from the **Data** view or with:

```bash
curl -s -X POST 'localhost:8080/api/v1/import?format=csv' \
  -H 'content-type: text/csv' \
  --data-binary @samples/mixed-accounts.csv
```

| File | What it exercises |
|------|-------------------|
| `minimal.csv` | One checking account and one balance |
| `mixed-accounts.csv` | Cash, savings, 401(k), Roth IRA, taxable brokerage, real estate, mortgage, and a crypto-style other asset |
| `retirement-accounts.csv` | 401(k), Traditional IRA, and Roth IRA with a balance history |
| `property-and-debt.csv` | Two properties, two mortgages, and a home-equity loan |

XLSX copies of every file are included. They are generated with the app's own
writer so every cell is text (dates and amounts stay exact):

```bash
cd backend
cargo run -p networth-adapters --example make_sample_xlsx -- ../samples
```

## Columns

```
schema_version, record_type, account_name, account_type, institution, currency,
ownership_scope, archived, as_of, amount, source
```

- `record_type` is `account` or `balance`.
- Account rows fill the account columns; balance rows fill `as_of` and `amount`.
- `account_name` is the stable key: re-importing the same file updates balances
  in place and creates no duplicate accounts.
- Amounts are decimal **text** (e.g. `1234.56`), never spreadsheet floats.
- `account_type` is one of: `401k`, `traditional_ira`, `roth_ira`,
  `taxable_brokerage`, `checking`, `savings`, `cash`, `real_estate`,
  `other_asset`, `mortgage`, `loan`.

## Notes

- XLSX works too (`?format=xlsx`); values are written and read as text so
  decimals stay exact. The CSV files here are the source of truth.
- Imports are validated in full first: if any row is invalid, nothing is
  written and the response lists the spreadsheet line and reason.
- Holdings (individual securities) are not part of this schema yet; they arrive
  with the investments capability.
