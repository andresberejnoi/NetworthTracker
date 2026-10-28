# Investments and market data

Investment accounts can hold individual securities. Each position is stored as
one or more **tax lots**, valued with a price, and included in net worth when
every holding in the account has a price.

## Model

- **Security** — symbol, name, asset class (`equity`, `fixed_income`,
  `real_estate`, `commodity`, `cash`, `other`), optional sector, currency.
- **Holding** — a security inside one account. Unique per account/security.
- **Lot** — acquisition date, quantity, and total cost basis. A holding's
  quantity and cost basis are the sum of its lots.
- **Price** — cached provider quote or a manual override, with an as-of
  timestamp and a stale flag.

Valuation:

```
market value   = sum(lot quantity) × unit price
unrealized P/L = market value − sum(lot cost basis)
```

## Net worth integration

If **every** holding in an account has a price in the reporting currency, the
account's holdings value replaces its recorded balance in net worth. If any
holding is unpriced, net worth falls back to the account's balance so the total
is never silently understated.

## API

```bash
# security
curl -s -X POST localhost:8080/api/v1/securities \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"VTI","name":"Total Market ETF","asset_class":"equity","sector":"other"}'

# holding with two lots
curl -s -X POST localhost:8080/api/v1/accounts/<id>/holdings \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"VTI","lots":[
        {"acquired_on":"2026-01-01","quantity":"10","cost_basis":"1000.00"},
        {"acquired_on":"2026-02-01","quantity":"5","cost_basis":"600.00"}]}'

# manual price
curl -s -X POST localhost:8080/api/v1/prices \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"VTI","price":"150.00"}'

# valuation
curl -s localhost:8080/api/v1/accounts/<id>/holdings -H "Authorization: Bearer $TOKEN"

# bulk lots (grid)
curl -s -X POST localhost:8080/api/v1/accounts/<id>/holdings/batch \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"rows":[{"symbol":"VTI","acquired_on":"2026-03-01","quantity":"2","cost_basis":"300.00"}]}'

# FX
curl -s -X POST localhost:8080/api/v1/fx-rates \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"base":"USD","quote":"EUR","rate":"0.92"}'
curl -s 'localhost:8080/api/v1/fx-rates?base=USD&quote=EUR' -H "Authorization: Bearer $TOKEN"
```

## Price add-ons (bring your own key)

No provider is bundled and none is required. Prices come from:

1. a **manual override** (always wins),
2. a fresh cached provider quote (15 minutes),
3. the last cached price, marked **stale**, or
4. nothing — the holding shows no value and net worth uses the balance.

A provider is an add-on the individual user enables with their own credentials:

```env
# .env (gitignored)
PRICE_PROVIDER=static
PRICE_STATIC_PRICES=AAPL=190.00,VTI=280.00
# Real adapters read their own user-supplied key:
PRICE_PROVIDER_API_KEY=your-own-key
```

- `PRICE_PROVIDER` unset or `none` → manual/cached prices only.
- `static` is a development add-on with a fixed table.
- A real provider is an adapter implementing the `PriceProvider` trait; it is
  compiled in and enabled by configuration, never loaded as third-party code.
- Credentials stay in `.env`, are never returned by the API, and are never
  committed. `GET /api/v1/prices/provider` reports only whether one is enabled.

## FX

FX rates are stored per currency pair with a timestamp. `FxService::convert`
uses the latest rate and records which rate was applied. v1 reports in USD;
holdings in other currencies are excluded from the net worth override until an
FX rate exists.

## Frontend

The **Investments** view lets you:

- add securities (symbol, name, asset class, sector),
- add or replace holdings with one or more lots,
- set manual prices,
- see positions, market value, unrealized P/L, and stale badges,
- see allocation by asset class and sector,
- paste lots into the bulk-entry grid.

Viewers see the data read-only.
