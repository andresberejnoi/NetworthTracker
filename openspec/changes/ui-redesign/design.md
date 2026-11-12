# Design

## Decisions

### D1. Desktop sidebar with a mobile bottom bar

**Decision:** Use a persistent left sidebar on desktop. On mobile, use a fixed
bottom bar with **Home, Accounts, Portfolio, Data, More**; More opens a sheet
containing Budget, Aggregation, and Settings. The sidebar becomes an off-canvas
drawer on tablets where the full sidebar does not fit.

**Rationale:** The current non-wrapping pill row overflows once optional/admin
views are present. A sidebar scales vertically, separates sections, and leaves
the content area for the actual task. A bottom bar keeps the four/five most-used
destinations thumb-reachable.

**Alternatives considered:** Top tabs with a More menu (rejected: still crowded
at laptop widths and weaker hierarchy); icon-only rail (rejected: harder to scan
for infrequent users); a single dashboard with everything below the fold
(rejected: poor discoverability and slow navigation).

### D2. Primary/secondary information architecture

**Decision:** Keep four primary destinations and move optional/admin features
into secondary sections.

```
Main
  Home
  Accounts
  Portfolio
  Data

Optional
  Budget
  Aggregation

Bottom
  Settings
```

- **Home** is the net worth dashboard.
- **Accounts** is the account list/detail and balance work.
- **Portfolio** has Investments and Crypto tabs.
- **Data** has Import/Export, Migration, and Documents tabs.
- **Budget** and **Aggregation** are optional; show them when enabled or
  configured, and keep them available from the secondary section.
- **Settings** contains Users & access, SSO, Aggregation configuration, Backups,
  and About/version.

**Rationale:** Daily work should be reachable in one click. Optional and
administrative tools should remain discoverable without competing with daily
tasks.

### D3. React Router, tested and React-18 compatible

**Decision:** Add `react-router-dom` v7.18.x, which supports React 18. Use
route-based views, a layout route for the authenticated shell, and nested routes
for Portfolio/Data/Settings sections.

**Rationale:** v7 is the current React Router line that still supports the
project's React 18 dependency; upgrading React to 19 solely for Router v8 adds
unrelated migration risk. The router is widely used and provides browser
back/forward, refresh-safe routes, deep links, and nested layouts.

**Alternatives considered:** staying on `useState` (rejected: no links/back
button); a custom hash router (rejected: unnecessary maintenance); Router v8
(rejected unless React 19 is separately adopted).

### D4. Home dashboard composition

**Decision:** Make Home a scannable net worth dashboard:

1. Page header with a greeting/context and one primary action.
2. Headline total net worth and period change.
3. Compact secondary metrics: liquid, liquidity, accessible.
4. Trend chart with a simple period/metric control.
5. Asset-class/account-type allocation.
6. Accounts needing attention or stale balances.
7. Recent activity and quick actions: Add balance, Take snapshot, Import data.

Age/tax/penalty/selling assumptions move into an **Assumptions** drawer or
Settings, not the main content. Backend version/status moves to Settings/About.

### D5. List-first pages, drawers for creation, detail routes

**Decision:** Accounts, holdings, crypto wallets, budgets, and documents use a
list/table as the page's primary content. Create/edit actions open a drawer or
modal; account detail uses a route.

Examples:

```text
/accounts                 list + Add account
/accounts/:id             detail, balances, history, edit
/portfolio/investments    list + Add lot/sale actions
/portfolio/crypto         wallets + Add wallet/balance actions
/data/import-export       import/export actions
/data/migration           migration wizard
/data/documents           encrypted document list
```

**Rationale:** Permanent forms consume attention and push primary content below
the fold. Drawers keep context while offering focused forms.

### D6. Route map

```text
/                                  Home dashboard
/accounts                          Account list
/accounts/:id                      Account detail
/portfolio/investments             Holdings and lots
/portfolio/crypto                  Crypto wallets and balances
/data/import-export                CSV/XLSX import/export
/data/migration                    Guided migration
/data/documents                    Documents
/budget                            Budget (optional/secondary)
/settings                          Settings landing
/settings/users                    Household members
/settings/sso                      SSO configuration/status
/settings/aggregation              Aggregation configuration
/settings/backups                  Backup status/actions/docs link
/settings/about                    Version, health, and diagnostics
/auth/oidc/callback                Existing OIDC callback route
```

The login/setup screen remains outside the authenticated shell. After login,
the router moves to Home; OIDC callbacks exchange the one-time code and then
navigate to Home.

### D7. Responsive and accessible shell

- Sidebar: 240px fixed on desktop; collapsible/off-canvas below 1024px.
- Bottom bar: visible below 768px, with safe-area padding and 44px touch targets.
- Content: max-width around 1200px, fluid gutters; page-level horizontal
  overflow is forbidden.
- Tables: wrap in a horizontal-scroll region; on phones, prioritize the name,
  value, and an actions menu. Do not shrink controls below usable size.
- Drawers/modals: focus trap, Escape closes, focus returns to trigger, labelled
  by their heading.
- Navigation: landmarks (`nav`, `main`), `aria-current="page"` on active routes,
  visible focus states, and skip-to-content.
- Respect `prefers-reduced-motion` for transitions.

### D8. Standardized presentation components

Build a small local set of primitives rather than adding a full component
library:

- `AppShell` (sidebar + topbar + content)
- `PageHeader` (title, context, primary/secondary actions)
- `Drawer`/`Modal` for focused creation/editing
- `EmptyState`
- `DataTable`/`TableScroller`
- `Field` wrapper with label, help, and error
- `Button` variants: primary, secondary, destructive, quiet
- `Tabs` for Portfolio/Data/Settings sections

Use the existing CSS variables and visual language; avoid a wholesale restyle
unless it is needed for hierarchy and responsive behavior.

## Route migration

The current `useState<View>` switch becomes nested route elements. Views can be
moved incrementally:

1. Add router and a shell route with a temporary "all views" mapping.
2. Move Home, Accounts, Portfolio, and Data.
3. Move Budget, Aggregation, and Settings.
4. Delete the old view state and conditional renderer.
5. Add deep-link/refresh/back-button tests.

React Query remains the data layer. Route params replace selected-item state
where practical; local drawer state remains local. API calls and query keys do
not change.

## Testing strategy

- Router tests with `MemoryRouter`: active item, deep links, nested tabs,
  back/forward behavior, unknown route/404.
- Role/feature visibility tests: viewer vs editor/owner; optional Budget and
  Aggregation; owner-only Settings.
- Mobile/sidebar tests: More sheet open/close, Escape/focus return, no
  page-level overflow using a narrow viewport.
- Form tests: create account drawer, validation, cancel/reset, successful save.
- Dashboard tests: headline figures, period change, chart, stale indicator,
  quick actions, assumption drawer.
- Accessibility checks for landmarks, labels, focus states, and keyboard
  reachability.
- Existing API/client tests remain unchanged; no backend changes.

## Risks / Trade-offs

- **Large UI refactor can destabilize every view** → migrate routes in stages
  and keep API/query behavior unchanged.
- **Sidebar plus mobile bar adds layout complexity** → centralize in one
  `AppShell` and test both breakpoints.
- **Router v7 with React 18** → pin v7 and avoid Router v8 until a separate
  React 19 migration.
- **Optional feature hiding can hurt discoverability** → keep optional entries
  visible in the secondary section when configured, and document how to enable
  them.
- **Drawers can be awkward on small screens** → use full-height sheets on
  phones and side drawers on desktop, with focus/keyboard handling.
