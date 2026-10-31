# Cryptocurrency tracking

Crypto uses two independent records:

- **Balances** — how much of an asset each wallet/bucket holds. These drive
  valuation and net worth.
- **Tax lots** — what was purchased, when, and for how much. These live in the
  shared `tax_lots` ledger and drive FIFO sales and tax reports. A balance can
  exist without lots; lots can exist without a synced address.

Keeping them separate is deliberate: a chain explorer can tell you the current
quantity but not what you paid, and a tax lot can exist for an asset held in a
wallet the app cannot query.

## Wallet modes

### Manual wallet / bucket

Use this for an exchange, a hardware wallet, or a single "BTC total" bucket.
The user records a dated quantity. No address or network access is required, so
this mode works fully offline.

### Address wallet

The user pastes a public address and selects the asset/chain. The app validates
the address and never accepts private keys or seed phrases. If a chain provider
is configured, the wallet can be synced to record a `chain` balance; otherwise
it remains a read-only address until a manual balance is entered.

Supported chains in this slice: **Bitcoin** and **Ethereum native assets**.
Tokens/ERC-20/SPL, NFTs, staking, and DeFi are out of scope.

## Providers

### Chain data (optional)

The app ships no chain credentials and no relay. Address sync is inert until a
provider is configured:

```env
# deterministic fake provider for demos/tests
CHAIN_PROVIDER=fake
CHAIN_FAKE_BALANCES=bitcoin:bc1q...=0.000000000000000001,ethereum:0x...=2.5
```

Real adapters implement the same `ChainDataProvider` trait. Recommended free
options:

- **Bitcoin:** `mempool.space` (no key).
- **Ethereum:** Blockscout (no key) or Etherscan V2 with a free user key
  (`CHAIN_PROVIDER=etherscan`, `CHAIN_PROVIDER_API_KEY=...`).
- `CHAIN_PROVIDER=public` uses mempool.space + Blockscout; `fake` is for
  tests/demos; unset is inert.
- **Solana:** public RPC or a free Helius key, when Solana is added.

The provider is optional. A sync failure never removes manual data; the latest
manual balance wins on the same date.

### Prices (optional)

Crypto prices reuse the existing price service:

1. manual price (always wins),
2. fresh provider cache (15 minutes),
3. provider quote,
4. last cached price, marked stale,
5. no price → the position is **unpriced** and excluded from the crypto value.

Manual prices are available at any time:

```bash
curl -s -X POST localhost:8080/api/v1/prices \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"BTC","price":"70000.00"}'
```

For a real price add-on, set `PRICE_PROVIDER=coinpaprika` for the keyless
CoinPaprika adapter (common crypto symbols), or use `static`/manual prices.
`PRICE_PROVIDER_API_KEY` is available for providers that need one. The app does
not bundle credentials.

## Totals, batches, and wallet identity

- A manual wallet is identified by **asset + label** (case-insensitive). So one
  `BTC / Coinbase` wallet stays one wallet no matter how many totals or lots you
  add.
- Multiple real wallets at the same exchange should use distinct labels, for
  example `Coinbase BTC 1` and `Coinbase BTC 2`. They remain separate wallets
  but aggregate into one BTC position for net worth.
- An address wallet is identified by **asset + address**. The same address
  cannot be registered twice under different labels.

A common starting point is a total-only position, such as `1.5 BTC` in
Coinbase. Record that as a dated wallet balance. To keep tax lots aligned,
use **Use balance as opening lot** in the crypto tax-lot section, enter the
opening cost basis (or mark it estimated), and save. Later small purchases can
be appended as separate purchase lots; the wallet remains one wallet and the
lots remain separate for FIFO.

Balances and lots are still independent records. The tax-lot section shows the
wallet balance, the remaining lots total, and a warning when they differ. This
prevents a silent mismatch between "how much I hold" and "what I paid".

For manual wallets, once lots exist, appending a purchase lot or recording a
sale updates the wallet's manual balance to the remaining lots total. A
total-only wallet is not overwritten by a batch until an opening lot is added,
so the initial total is preserved while you decide its basis.

## API

```bash
# asset
curl -s -X POST localhost:8080/api/v1/crypto/assets \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"symbol":"BTC","name":"Bitcoin","chain":"bitcoin","decimals":18}'

# manual wallet
curl -s -X POST localhost:8080/api/v1/crypto/wallets \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"asset_id":"<id>","kind":"manual","chain":"bitcoin","label":"Coinbase"}'

# manual balance
curl -s -X POST localhost:8080/api/v1/crypto/wallets/<wallet>/balances \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"as_of":"2026-01-01","quantity":"0.003"}'

# address wallet + sync
curl -s -X POST localhost:8080/api/v1/crypto/wallets \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"asset_id":"<id>","kind":"address","chain":"bitcoin","label":"Cold","address":"bc1..."}'
curl -s -X POST localhost:8080/api/v1/crypto/wallets/<wallet>/sync \
  -H "Authorization: Bearer $TOKEN"

# aggregated positions
curl -s 'localhost:8080/api/v1/crypto/positions?as_of=2026-01-01' \
  -H "Authorization: Bearer $TOKEN"
```

## Net worth

Each crypto asset with a price becomes a synthetic `crypto` account in the net
worth engine. Crypto is treated as **marketable** (included in liquid net worth,
not in cash-on-hand liquidity). Accessible net worth uses the full crypto value
in this slice; capital-gains tax/selling-cost haircuts are a later policy.

Unpriced crypto is excluded from the value and flagged in the UI. It is never
silently valued at zero.

## UI

The **Crypto** view supports:

- adding assets and manual/address wallets,
- recording dated manual balances,
- syncing address wallets when a chain add-on is configured,
- setting manual prices,
- aggregated positions by asset,
- bulk wallet totals, grouped by asset/label or address so repeated rows do
  not create duplicate wallets (each dated row is a balance observation),
- bulk purchase lots, where each row is a separate transaction/lot and
  multiple rows for the same wallet are preserved for FIFO/tax reporting.

## Tax lots

Crypto purchase lots and FIFO sales use the same shared `tax_lots` /
`tax_sales` / `tax_disposals` ledger as securities. The Crypto view has a
wallet-scoped tax-lot section: append purchase lots (with actual or estimated
basis), record FIFO sales, and review realized gains by disposal and term.
Balances and lots remain separate records; a wallet can have one without the
other.

## Limitations

- Bitcoin and Ethereum native assets only.
- Household wallets only; private crypto wallets are rejected for now.
- Current chain balances and transaction history, not reconstructed historical
  on-chain balances.
- Solana and token/ERC-20 adapters are follow-ups; Bitcoin and Ethereum native
  public adapters plus the keyless CoinPaprika price adapter are included.
- The JSON backup does not yet include crypto/investment detail; the full
  `pg_dump` backup does.
