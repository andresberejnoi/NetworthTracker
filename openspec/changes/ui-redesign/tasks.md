# Tasks

## 1. Router and application shell

- [ ] 1.1 Add `react-router-dom` v7 (React 18-compatible) and a route tree for the authenticated app
- [ ] 1.2 Add an authenticated layout route and keep login/setup/OIDC callback outside the shell
- [ ] 1.3 Build the desktop sidebar, top bar, and content shell with active-route and `aria-current` states
- [ ] 1.4 Build the mobile bottom bar, More sheet, off-canvas sidebar, and safe-area layout
- [ ] 1.5 Add a Not Found route and a route-level error/loading presentation
- [ ] 1.6 Remove the old `useState<View>` view switch and convert all destinations to URLs

## 2. Information architecture and visibility

- [ ] 2.1 Limit primary navigation to Home, Accounts, Portfolio, and Data
- [ ] 2.2 Add the secondary Optional section for Budget and Aggregation
- [ ] 2.3 Group Users, SSO, Aggregation configuration, Backups, and About under Settings
- [ ] 2.4 Make navigation role-aware and feature-aware without hiding configured optional features
- [ ] 2.5 Add deep-link, refresh, back/forward, and unknown-route tests

## 3. Home dashboard

- [ ] 3.1 Build the Home page header with one primary action and a small set of quick actions
- [ ] 3.2 Add headline total net worth, period change, and compact liquid/liquidity/accessible metrics
- [ ] 3.3 Add trend chart controls and allocation breakdown
- [ ] 3.4 Add an accounts-needing-attention/stale-data summary with links to accounts
- [ ] 3.5 Move age/tax/penalty/selling assumptions into a secondary drawer or Settings
- [ ] 3.6 Move backend version/health metadata from the old Overview page to Settings/About
- [ ] 3.7 Add role-aware dashboard tests and empty/error/loading states

## 4. Accounts and forms

- [ ] 4.1 Make Accounts a list-first page with search/filter and one Add account action
- [ ] 4.2 Move account creation/editing into a drawer or modal with progressive disclosure
- [ ] 4.3 Add `/accounts/:id` detail with balances, history, and edit actions
- [ ] 4.4 Move balance entry into a focused drawer/modal shared by Home and Accounts
- [ ] 4.5 Standardize field labels, help text, validation, and cancel/reset behavior
- [ ] 4.6 Add account-list/detail/drawer tests

## 5. Portfolio and Data consolidation

- [ ] 5.1 Move Investments and Crypto under `/portfolio/*` with accessible tabs
- [ ] 5.2 Move holding/lot and crypto wallet/balance creation into focused drawers
- [ ] 5.3 Move import/export, guided migration, and documents under `/data/*` with accessible tabs
- [ ] 5.4 Preserve owner-only document access and existing migration/document behavior
- [ ] 5.5 Add Portfolio/Data tab and deep-link tests

## 6. Optional features and Settings

- [ ] 6.1 Keep Budget available from the Optional section and provide an empty/disabled explanation
- [ ] 6.2 Move Aggregation configuration/status under Settings and the Optional menu
- [ ] 6.3 Add Settings pages for Users, SSO, Aggregation, Backups, and About
- [ ] 6.4 Keep SSO setup/link guidance discoverable without exposing secrets
- [ ] 6.5 Add role/feature visibility tests for optional and owner-only destinations

## 7. Presentation system and accessibility

- [ ] 7.1 Add shared `PageHeader`, `Drawer`, `EmptyState`, `DataTable`, `Field`, and button primitives
- [ ] 7.2 Apply the shared header/primary-action pattern across every view
- [ ] 7.3 Make tables and wide content scroll within their region without page-level overflow
- [ ] 7.4 Add landmarks, skip-to-content, focus states, focus return, Escape handling, and reduced-motion support
- [ ] 7.5 Verify keyboard-only navigation and narrow/mobile viewport layouts

## 8. Verification and docs

- [ ] 8.1 Add responsive shell/navigation/forms/dashboard tests to the frontend suite
- [ ] 8.2 Run the full frontend and release matrix
- [ ] 8.3 Add a short UI navigation guide and update screenshots/README/CHANGELOG
- [ ] 8.4 Run the release process and tag the next version
