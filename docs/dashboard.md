# Dashboard

The dashboard lives in `frontend/` and is a standalone consumer of the backend
HTTP API. It never imports backend code.

## Views

| View | What it shows |
|------|---------------|
| Overview | Backend service and version (liveness) |
| Net worth | Summary cards, history chart, breakdown |
| Accounts | Create, list, and archive accounts |
| Bulk entry | Spreadsheet-like grid with paste and per-row results |
| Data | Import/export CSV/XLSX and download templates |
| Users | Household members (owners only) |

When authentication is enabled, the app shows a first-run setup screen or a
login screen before any of these views. See `docs/access-control.md`.

## Net worth

- **Four summary cards** — total, liquid, liquidity, and accessible net worth, each with a one-line
  explanation. Values are formatted for display, but the underlying decimals come from the API as
  exact strings.
  - *Total* — assets minus liabilities.
  - *Liquid* — cash, deposit accounts, brokerage, and retirement accounts (quickly sellable).
  - *Liquidity* — cash and cash-like equivalents available right now.
  - *Accessible* — net proceeds after estimated taxes, penalties, and selling costs.
- **History chart** — an SVG line chart drawn from stored snapshots. A metric
  selector switches between total, liquid, and accessible. Fewer than two
  snapshots shows a prompt instead of a misleading flat line.
- **Take snapshot** — persists an immutable point in time. Later balance or
  policy changes do not rewrite it.
- **Breakdown** — a table grouped by asset class or account type, showing the
  net amount per group. A selector switches the grouping.
- **Owner age** — feeds the early-withdrawal penalty used by accessible net
  worth, so changing it immediately updates the accessible figure.
- **Freshness badges** — `manual entries` appears when any contributing balance
  was typed by hand; `stale balances` appears when a balance is older than the
  freshness threshold (45 days).

## Data flow

```
React components
   └── src/api/client.ts   (typed from contracts/openapi.yaml)
         └── fetch → http://localhost:8080/api/v1/*
```

The base URL comes from `VITE_API_BASE_URL` and is baked into the frontend
container at build time. The backend allows the frontend origin via CORS.

## Tests

- Component tests (`npm test`) mock the API and cover the summary, breakdown
  grouping, chart rendering, freshness badges, and snapshot creation.
- The cross-boundary integration test (`make integration-test`) drives the real
  backend and Postgres through the same client.

## Adding a view

1. Add the endpoint and schema to `contracts/openapi.yaml`.
2. Regenerate the typed client (`make frontend-api`).
3. Add the client function in `src/api/client.ts`.
4. Build the view under `src/features/<name>/` with component tests.
5. Add it to the nav in `App.tsx`.
6. Extend the integration test if the feature spans the boundary.
