# Spec Delta

## Purpose

Provides bulk, spreadsheet-based data entry and portability through CSV/XLSX import and export,
plus local backup and restore of the whole dataset.

## ADDED Requirements

### Requirement: CSV and XLSX import
The system SHALL import accounts, balances, and holdings from CSV or XLSX files using a documented
template that the user can edit in any spreadsheet application.

#### Scenario: Importing a balances template
- **WHEN** a user imports a spreadsheet containing accounts and dated balances
- **THEN** the accounts and balances are created or updated in the household

#### Scenario: Importing holdings
- **WHEN** a user imports a spreadsheet listing security holdings with symbols and quantities
- **THEN** the holdings are attached to the referenced investment accounts

### Requirement: Validated import with row-level errors
The system SHALL validate an import before applying it and SHALL report actionable row-level errors,
such as an unknown account type or an unparseable amount, without applying a partially valid file.

#### Scenario: Invalid file rejected
- **WHEN** an import contains a row with an invalid amount
- **THEN** the import is rejected as a whole and the user is shown the row and the reason

#### Scenario: Validation precedes writes
- **WHEN** any validation error is present
- **THEN** no records from that file are written to the database

### Requirement: Idempotent re-import
Re-importing the same file SHALL NOT create duplicate accounts, balances, or holdings, deduplicating
on a stable record key.

#### Scenario: Import twice
- **WHEN** a user imports the same file twice
- **THEN** the second import updates existing records rather than duplicating them

### Requirement: Data export
The system SHALL export all household data, including accounts, balances, holdings, and
configuration, to CSV or XLSX files.

#### Scenario: Exporting for backup or editing
- **WHEN** a user exports the household data
- **THEN** a spreadsheet-compatible file is produced containing the household's accounts, balances, and holdings

### Requirement: Backup and restore
The system SHALL produce a complete local backup of its database and SHALL restore from such a
backup, on the same machine or a replacement machine.

#### Scenario: Restore on a new machine
- **WHEN** a user restores a backup on a different machine with no network connection
- **THEN** all accounts, balances, holdings, and history are available exactly as at backup time

### Requirement: Exact decimal round-trip
Monetary and cryptocurrency amounts SHALL round-trip through import, export, and backup without any
loss of precision.

#### Scenario: High-precision value survives
- **WHEN** a value with many decimal places is exported and then re-imported
- **THEN** the re-imported value is identical to the original with no rounding
