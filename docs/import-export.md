# Import and export

Accounts and balances move in and out through one **versioned table format**,
available as CSV or XLSX. The same schema is used by the API, the **Data** view,
and the sample files in `samples/`.

## Schema (version 1)

```
schema_version, record_type, account_name, account_type, institution, currency,
ownership_scope, archived, as_of, amount, source
```

| Column | Used by | Meaning |
|--------|---------|---------|
| `schema_version` | both | Must be `1` |
| `record_type` | both | `account` or `balance` |
| `account_name` | both | Stable key; balance rows reference the account by name |
| `account_type` | account | `401k`, `traditional_ira`, `roth_ira`, `taxable_brokerage`, `checking`, `savings`, `cash`, `real_estate`, `other_asset`, `mortgage`, `loan` |
| `institution` | account | Optional |
| `currency` | both | ISO code; defaults to `USD` (balances default to the account's currency) |
| `ownership_scope` | account | `household` (default) or `user` |
| `archived` | account | `true`/`false` (default false) |
| `as_of` | balance | `YYYY-MM-DD` |
| `amount` | balance | Decimal **text**, e.g. `1234.56` |
| `source` | balance | `manual` (default), `import`, or `provider` |

Example:

```csv
schema_version,record_type,account_name,account_type,institution,currency,ownership_scope,archived,as_of,amount,source
1,account,Everyday Checking,checking,Local Bank,USD,household,false,,,
1,balance,Everyday Checking,,,,,,2026-10-17,2500.00,manual
```

## Endpoints

```bash
# import (format from ?format= or Content-Type)
curl -s -X POST 'localhost:8080/api/v1/import?format=csv' \
  -H 'content-type: text/csv' \
  --data-binary @samples/mixed-accounts.csv

# export
curl -s 'localhost:8080/api/v1/export?format=csv' -o networth.csv
curl -s 'localhost:8080/api/v1/export?format=xlsx' -o networth.xlsx

# starter template
curl -s 'localhost:8080/api/v1/template?format=csv' -o template.csv
```

Import response:

```json
{
  "accounts_created": 3,
  "balances_saved": 10,
  "errors": []
}
```

If `errors` is non-empty, **nothing was written**. Each error carries the
spreadsheet line number:

```json
{ "accounts_created": 0, "balances_saved": 0,
  "errors": [{ "row": 3, "error": "invalid amount: oops" }] }
```

## Guarantees

- **Validated in full before writing.** A single bad row rejects the import.
- **Idempotent.** Accounts are matched by name; balances are upserted per
  account and day. Re-importing the same file creates no duplicates.
- **Exact decimals.** Amounts are text end to end; `0.000000000000000001`
  round-trips unchanged.
- **XLSX is written as text** so Excel's binary floats never touch money values.
  For the same reason, prefer text-formatted amount cells if you author XLSX by
  hand.

## Samples

`samples/` contains ready-to-import files covering different account types:

- `minimal.csv`
- `mixed-accounts.csv` (cash, retirement, brokerage, property, mortgage, crypto-style)
- `retirement-accounts.csv`
- `property-and-debt.csv`

## Backups

`GET /api/v1/backup` produces a complete JSON backup **including snapshot
history and the redemption policy used**. `POST /api/v1/backup/restore` restores
it (owner only):

```bash
curl -s localhost:8080/api/v1/backup -H "Authorization: Bearer $TOKEN" > backup.json
curl -s -X POST localhost:8080/api/v1/backup/restore \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  --data-binary @backup.json
```

Restore is idempotent: accounts match by name, balances upsert per day, and
identical snapshots are skipped. Validation runs before anything is written.
Private accounts carry their owner's **username**, so the same household members
must exist on the target installation.

For a raw database dump instead:

```bash
docker compose exec db pg_dump -U networth networth > networth-backup.sql
```
