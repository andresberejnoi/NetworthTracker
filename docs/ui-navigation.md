# Navigation and UI structure

The app uses a route-based shell with a desktop sidebar and a mobile bottom
navigation bar. The goal is to keep four daily destinations visible and move
optional/admin tools out of the primary path.

## Desktop

```text
Main
  Home
  Accounts
  Portfolio     -> Investments | Crypto
  Data          -> Import/Export | Migration | Documents

Optional
  Budget

Settings
  Users & access
  SSO
  Aggregation
  Backups
  About
```

## Mobile

The bottom bar contains:

```text
Home   Accounts   Portfolio   Data   More
```

**More** opens a sheet with Budget, Aggregation, and Settings; Aggregation and the rest of the admin views live under Settings.

## Routes

| Route | View |
|---|---|
| `/` | Home net worth dashboard |
| `/accounts` | Account list |
| `/accounts/:id` | Account detail and balance history |
| `/accounts/bulk` | Bulk balance entry |
| `/portfolio/investments` | Holdings, lots, sales |
| `/portfolio/crypto` | Crypto wallets, balances, lots |
| `/data/import-export` | Spreadsheet import/export |
| `/data/migration` | Guided CSV/JSON migration |
| `/data/documents` | Encrypted documents (owner) |
| `/budget` | Optional budget planning |
| `/settings/users` | Household members (owner) |
| `/settings/sso` | SSO/OIDC status and linking (owner) |
| `/settings/aggregation` | Aggregation configuration (owner) |
| `/settings/backups` | Backup commands and guides (owner) |
| `/settings/about` | Version/API status |

Login, setup, and `/auth/oidc/callback` remain outside the authenticated shell.

## Interaction patterns

- **List first:** Accounts, holdings, wallets, documents, and members are lists;
  creation/editing opens a focused drawer instead of a permanent form.
- **One primary action per region:** page headers expose the main next action and
  keep secondary actions visually subordinate.
- **Progressive disclosure:** advanced/rare fields are in drawers or collapsed
  controls. Planning assumptions are in a secondary panel on Home.
- **Responsive tables:** table regions scroll or collapse; the page itself must
  not overflow.
- **Keyboard and focus:** drawers close on Escape and return focus; active
  routes use `aria-current`; navigation has a skip-to-content link.

The router is `react-router-dom` v7, chosen because the frontend is on React 18
and Router v8 requires React 19.
