# Tasks

## 1. Router and application shell

- [x] 1.1 Add `react-router-dom` v7 (React 18-compatible) and a route tree for the authenticated app
- [x] 1.2 Add an authenticated layout route and keep login/setup/OIDC callback outside the shell
- [x] 1.3 Build the desktop sidebar, top bar, and content shell with active-route and `aria-current` states
- [x] 1.4 Build the mobile bottom bar, More sheet, off-canvas sidebar, and safe-area layout
- [x] 1.5 Add a Not Found route and a route-level error/loading presentation
- [x] 1.6 Remove the old `useState<View>` view switch and convert all destinations to URLs

## 2. Information architecture and visibility

- [x] 2.1 Limit primary navigation to Home, Accounts, Portfolio, and Data
- [x] 2.2 Add the secondary Optional section for Budget and Aggregation
- [x] 2.3 Group Users, SSO, Aggregation configuration, Backups, and About under Settings
- [x] 2.4 Make navigation role-aware and feature-aware without hiding configured optional features
- [x] 2.5 Add deep-link, refresh, back/forward, and unknown-route tests

## 3. Home dashboard

- [x] 3.1 Build the Home page header with one primary action and a small set of quick actions
- [x] 3.2 Add headline total net worth, period change, and compact liquid/liquidity/accessible metrics
- [x] 3.3 Add trend chart controls and allocation breakdown
- [x] 3.4 Add an accounts-needing-attention/stale-data summary with links to accounts
- [x] 3.5 Move age/tax/penalty/selling assumptions into a secondary drawer or Settings
- [x] 3.6 Move backend version/health metadata from the old Overview page to Settings/About
- [x] 3.7 Add role-aware dashboard tests and empty/error/loading states

## 4. Accounts and forms

- [x] 4.1 Make Accounts a list-first page with search/filter and one Add account action
- [x] 4.2 Move account creation/editing into a drawer or modal with progressive disclosure
- [x] 4.3 Add `/accounts/:id` detail with balances, history, and edit actions
- [x] 4.4 Move balance entry into a focused drawer/modal shared by Home and Accounts
- [x] 4.5 Standardize field labels, help text, validation, and cancel/reset behavior
- [x] 4.6 Add account-list/detail/drawer tests

## 5. Portfolio and Data consolidation

- [x] 5.1 Move Investments and Crypto under `/portfolio/*` with accessible tabs
- [x] 5.2 Move holding/lot and crypto wallet/balance creation into focused drawers
- [x] 5.3 Move import/export, guided migration, and documents under `/data/*` with accessible tabs
- [x] 5.4 Preserve owner-only document access and existing migration/document behavior
- [x] 5.5 Add Portfolio/Data tab and deep-link tests

## 6. Optional features and Settings

- [x] 6.1 Keep Budget available from the Optional section and provide an empty/disabled explanation
- [x] 6.2 Move Aggregation configuration/status under Settings and the Optional menu
- [x] 6.3 Add Settings pages for Users, SSO, Aggregation, Backups, and About
- [x] 6.4 Keep SSO setup/link guidance discoverable without exposing secrets
- [x] 6.5 Add role/feature visibility tests for optional and owner-only destinations

## 7. Presentation system and accessibility

- [x] 7.1 Add shared `PageHeader`, `Drawer`, `EmptyState`, `DataTable`, `Field`, and button primitives
- [x] 7.2 Apply the shared header/primary-action pattern across every view
- [x] 7.3 Make tables and wide content scroll within their region without page-level overflow
- [x] 7.4 Add landmarks, skip-to-content, focus states, focus return, Escape handling, and reduced-motion support
- [x] 7.5 Verify keyboard-only navigation and narrow/mobile viewport layouts

## 8. Verification and docs

- [x] 8.1 Add responsive shell/navigation/forms/dashboard tests to the frontend suite
- [x] 8.2 Run the full frontend and release matrix
- [x] 8.3 Add a short UI navigation guide and update screenshots/README/CHANGELOG
- [x] 8.4 Run the release process and tag the next version
