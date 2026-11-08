# Guided data migration

Guided migration imports a generic CSV or JSON portfolio through the HTTP API.
It runs locally: no data is sent to a service. The flow is preview first, then an
explicit apply after a verified encrypted backup.

## Quick start

```bash
# Preview a sample portfolio (writes nothing)
./scripts/migrate.sh --file samples/migration-portfolio.json

# Apply it; the script takes a verified backup first
./scripts/migrate.sh --file samples/migration-portfolio.json --apply
```

With authentication enabled, pass a session token:

```bash
AUTH_TOKEN=... ./scripts/migrate.sh --file portfolio.json --apply
# or: ./scripts/migrate.sh --file portfolio.json --apply --token ...
```

`--source generic_csv|generic_json` overrides extension inference
(`.csv`/`.json`). `--mapping mapping.json` overrides default source keys.
`--partial` writes valid rows and reports rejected rows; without it, any invalid
row rejects the entire atomic apply.

## Supported records

The generic adapter recognizes these `record_type` values:

| `record_type` | Required fields |
|---|---|
| `account` | `account_name`, `account_type`; optional institution/currency/scope/archived |
| `balance` | `account_name`, `as_of`, `amount`; optional source/currency |
| `holding_lot` | `account_name`, `symbol`, `acquired_on`, `quantity`, `cost_basis`; optional security name/asset class/fees/lot source/basis |
| `crypto_wallet` | `asset_symbol`, `chain`, `wallet_label`; optional kind/address/scope |
| `crypto_balance` | crypto wallet fields plus `as_of`, `quantity`; optional source |
| `crypto_lot` | crypto wallet fields plus `acquired_on`, `quantity`, `cost_basis`; optional fees/basis |

Default source keys are the field names above. `mapping` maps a normalized field
to a different source column/object key:

```json
{
  "account_name": "Account",
  "as_of": "Date",
  "amount": "Balance"
}
```

Amounts and quantities are decimal text; floats are never used. The importer
validates every row and returns row-level errors with source row numbers.

## Stable keys and idempotency

- Accounts match by case-insensitive name.
- Balances upsert per account/day.
- Existing investment holdings are not duplicated; lots are skipped for a
  holding that already existed before the run.
- Securities match by ticker; crypto assets by symbol.
- Crypto wallets match manual wallets by asset + label and address wallets by
  asset + address. Existing wallets keep their lots; balances still upsert.

Re-running the same file therefore does not duplicate accounts, holdings, or
wallets. Rejected rows are reported in `errors`.

## Backup requirement

`POST /api/v1/migrations/apply` refuses requests with
`backup_confirmed: false`. `scripts/migrate.sh --apply` calls
`scripts/backup.sh` first, which takes and verifies the encrypted backup. If the
backup fails, the migration is not sent.

This is a safety gate, not a substitute for testing the restore path. See
[`backup.md`](backup.md).

## Provenance

Each apply creates a `migration_runs` row and `migration_provenance` links for
written accounts, holdings, crypto wallets, and crypto lots. The migration report
returns the `migration_id`; the source/time links are available for audit and
future document-to-record provenance.

## API

```bash
curl -s -X POST localhost:8080/api/v1/migrations/preview \
  -H 'content-type: application/json' \
  --data-binary @request.json
```

`request.json` is `{ "source": "generic_json", "mapping": {}, "content": "[...]" }`.
The apply endpoint uses the same fields plus `"mode": "atomic"|"partial"` and
`"backup_confirmed": true`.

## Samples

- `samples/migration-portfolio.json`
- `samples/migration-portfolio.csv`

These cover accounts, balances, an investment lot, a crypto wallet/balance, and
a crypto lot.
