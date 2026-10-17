# HTTP API

The backend is a standalone JSON API and the only integration point between
`backend/` and `frontend/`. The contract is `contracts/openapi.yaml`; the
frontend's typed client is generated from it.

- Base URL (local): `http://localhost:8080`
- All request and response bodies are JSON.
- **Monetary amounts are decimal strings**, never numbers:
  `{"amount": "250.50", "currency": "USD"}`.

## System

```bash
curl -s localhost:8080/health
# {"status":"ok"}

curl -s localhost:8080/api/v1/ping
# {"message":"pong","service":"NetworthTracker","version":"0.1.20261017"}
```

## Accounts

Create an account:

```bash
curl -s -X POST localhost:8080/api/v1/accounts \
  -H 'content-type: application/json' \
  -d '{"name":"Fidelity 401k","account_type":"401k","institution":"Fidelity"}'
```

```json
{
  "id": "6f1c…",
  "name": "Fidelity 401k",
  "account_type": "401k",
  "institution": "Fidelity",
  "currency": "USD",
  "ownership_scope": "household",
  "archived": false
}
```

`account_type` is one of:

```
401k  traditional_ira  roth_ira  taxable_brokerage  checking  savings
cash  real_estate  other_asset  mortgage  loan
```

`currency` defaults to `USD`; `ownership_scope` defaults to `household`
(`user` makes the account private once identity lands).

List and fetch:

```bash
curl -s localhost:8080/api/v1/accounts
curl -s 'localhost:8080/api/v1/accounts?include_archived=true'
curl -s localhost:8080/api/v1/accounts/<id>
```

Archive (soft close; history is preserved):

```bash
curl -s -X POST localhost:8080/api/v1/accounts/<id>/archive -o /dev/null -w '%{http_code}\n'
# 204
```

## Balances

Record a dated balance (one value per account per day; re-posting the same date
replaces it):

```bash
curl -s -X POST localhost:8080/api/v1/accounts/<id>/balances \
  -H 'content-type: application/json' \
  -d '{"as_of":"2026-10-17","amount":"250.50"}'
```

```json
{
  "id": "d0a2…",
  "account_id": "6f1c…",
  "as_of": "2026-10-17",
  "amount": "250.50",
  "currency": "USD",
  "source": "manual"
}
```

`amount` accepts arbitrary-precision decimals; for example
`"0.000000000000000001"` round-trips unchanged. `currency` defaults to the
account's currency and must match it. `source` is `manual`, `import`, or
`provider`.

List (newest first):

```bash
curl -s localhost:8080/api/v1/accounts/<id>/balances
```

## Errors

Errors are `{"error": "<message>"}` with a 4xx/5xx status:

```json
{ "error": "unknown account type: beanie_babies" }
```

| Status | Meaning |
|--------|---------|
| 400    | Validation failure (bad type, blank name, date, currency mismatch) |
| 404    | Account not found |
| 409    | Conflict |
| 500    | Unexpected storage failure |

## Verifying

```bash
make integration-test   # real Postgres + backend, driven through the frontend client
make backend-test-db    # repository tests against a throwaway Postgres
```
