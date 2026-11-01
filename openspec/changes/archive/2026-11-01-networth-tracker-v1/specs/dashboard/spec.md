# Spec Delta

## Purpose

Presents the household's net worth and related metrics through a modern web interface with history
charts, filtering by account type and asset class, and clear indicators of data freshness.

## ADDED Requirements

### Requirement: Net worth summary
The dashboard SHALL display current total, liquid, and accessible net worth for the selected scope,
along with the change over a selectable period.

#### Scenario: Summary shown on load
- **WHEN** a user opens the dashboard
- **THEN** total, liquid, and accessible net worth are shown for the current period along with the change from the previous period

#### Scenario: Period selection
- **WHEN** a user selects a different period, such as year to date or last twelve months
- **THEN** the summary and its change values recompute for that period

### Requirement: Net worth history chart
The dashboard SHALL display a time-series chart of net worth history, and SHALL let the user choose
which net worth form, total, liquid, or accessible, is plotted.

#### Scenario: Charting history
- **WHEN** snapshots exist across multiple dates
- **THEN** the dashboard plots net worth over time and the user can switch between the net worth forms

### Requirement: Filtering and breakdown
The dashboard SHALL allow the user to filter and break down metrics by account type and by asset
class or sector, including real estate, equities, bonds, cash, and cryptocurrency.

#### Scenario: Filter by account type
- **WHEN** a user filters to retirement accounts
- **THEN** the summary and chart reflect only accounts of those types

#### Scenario: Breakdown across asset classes
- **WHEN** a user views the allocation breakdown
- **THEN** the dashboard shows the share of net worth by asset class or sector

### Requirement: Household and per-user views
The dashboard SHALL let a user view the whole household or only their own accounts, subject to the
user's permissions.

#### Scenario: Scoped viewing
- **WHEN** a user without access to another member's private accounts views the dashboard
- **THEN** those accounts do not appear in any metric or chart

### Requirement: Local and offline operation
The dashboard SHALL be served from the local installation and SHALL remain functional without
internet access, using stored balances and cached or manual prices.

#### Scenario: Dashboard offline
- **WHEN** the machine has no network connection
- **THEN** the dashboard still loads and displays net worth from stored data

### Requirement: Data freshness indicators
The dashboard SHALL indicate when a displayed value is based on stale or manually entered data
rather than a fresh quote.

#### Scenario: Stale value marked
- **WHEN** a value is computed from a cached price or a manual entry
- **THEN** the dashboard marks that value or its data as stale or manual
