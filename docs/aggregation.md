# Read-only account aggregation

Aggregation is an optional add-on. The app ships no provider credentials and
operates no relay; the user supplies their own provider token, which is
encrypted locally and sent only to that provider.

## Interface

`AggregationProvider` is intentionally read-only:

```rust
#[async_trait]
pub trait AggregationProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn fetch(&self, token: &str) -> Result<ProviderSnapshot, ProviderError>;
}
```

It can only return accounts, balances, and holdings. There are no transfer,
trade, payment, or other write methods.

The bundled `fake` provider is for tests and demos. Enable it with:

```env
AGGREGATION_PROVIDER=fake
AGGREGATION_ENCRYPTION_KEY=<base64 32-byte key>
```

Generate a local key, for example:

```bash
python3 -c 'import base64,os; print(base64.b64encode(os.urandom(32)).decode())'
```

Real providers implement the same trait and use the user's own subscription or
API key.

## Provenance

Every sync writes provider balances and holdings into dedicated provenance
tables (`aggregation_balances`, `aggregation_holdings`) with a sync timestamp.
Provider data is also written to the linked local account as a `provider`
balance **only when there is no manual balance for that date**. A manual value
always wins and is retained, and the UI shows which value is in effect.

If the provider is unreachable, the sync fails gracefully and existing manual
and provider data remain available.

## API

```bash
# configure (owner only); omit token to keep the stored one
curl -s -X POST localhost:8080/api/v1/aggregation/config \
  -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"provider":"fake","enabled":true,"token":"your-own-token"}'

# sync
curl -s -X POST localhost:8080/api/v1/aggregation/sync \
  -H "Authorization: Bearer $TOKEN"

# status, links, provenance, and provider holdings
curl -s localhost:8080/api/v1/aggregation \
  -H "Authorization: Bearer $TOKEN"
```

## UI

The owner-only **Aggregation** view lets the user choose a provider, enter
their own token, enable/disable the add-on, sync, and see provider versus local
balances, which value is in effect, the last sync time, and provider holdings.

## Limits

- The only bundled adapter is the deterministic fake; real bank/brokerage
  adapters are follow-ups behind the same read-only trait.
- Tokens are encrypted at rest with AES-256-GCM; the key lives in `.env` and is
  not committed.
- Provider holdings are provenance data; they do not create tax lots or
  silently replace manual holdings.
