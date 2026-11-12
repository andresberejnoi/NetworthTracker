# Proposal

## Why

The current interface exposes eleven owner-level top navigation items, has no
route-based navigation, and puts several creation forms and advanced controls
into permanent page chrome. The top tabs overflow at narrower widths and the
screen does not make the next action obvious. A redesign should make the
household's financial position the home screen and keep daily tasks easy to
reach on both desktop and mobile.

## What Changes

- Replace the current pill-row/state-based navigation with a route-based
  application shell: a persistent desktop sidebar, a mobile bottom bar with a
  More sheet, and deep-linkable URLs with browser back/forward.
- Make **Home** the net worth dashboard: headline figures, trend, allocation,
  account summary, freshness indicators, and quick actions.
- Reduce primary navigation to **Home, Accounts, Portfolio, Data**.
- Move optional features (Budget and Aggregation) into a secondary/sidebar
  menu, and admin/configuration (Users, SSO, Backups, About) into Settings.
- Group Investments and Crypto under **Portfolio**.
- Group import/export, guided migration, and document ingestion under **Data**.
- Make Accounts list-first, with create/edit forms in a drawer or modal and a
  dedicated account detail route.
- Move advanced assumptions and dense controls behind progressive disclosure;
  one primary action per page region.
- Standardize page headers, empty states, tables, form fields, and error states.
- Ensure the layout is responsive and usable on mobile, including safe-area
  handling and no page-level horizontal overflow.
- Add route/navigation tests, responsive tests, and accessibility checks.

No API or data-model changes are required. This is a frontend shell and
information-architecture change.

## Capabilities

### New Capabilities

- `app-shell`: route-based responsive navigation, sidebar/mobile shell,
  primary/secondary menu organization, page-header patterns, and accessible
  progressive-disclosure forms.

### Modified Capabilities

- `dashboard`: Home becomes the net worth dashboard with headline figures,
  trend, allocation, freshness, and quick actions, with advanced assumptions
  kept out of the primary scan path.

## Impact

- `frontend/src/App.tsx`, `frontend/src/App.test.tsx`, and all view entry points.
- Add `react-router-dom` v7 (React 18-compatible) and route definitions.
- New shell components: sidebar, top bar, mobile bottom bar, More sheet,
  page header, drawer, empty state, and responsive table primitives.
- Split `NetWorthView` into reusable dashboard sections and move account-create
  forms into drawers.
- Consolidate navigation for Investments/Crypto, import/migration/documents,
  Budget/Aggregation, and owner settings.
- CSS/layout changes in `frontend/src/index.css`; no backend or OpenAPI changes.
- Tests for routing, role/feature visibility, responsive nav, forms, and
  keyboard/focus behavior.
