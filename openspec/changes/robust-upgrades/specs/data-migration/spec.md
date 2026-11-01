# Spec Delta

## Purpose

Lets a user bring an existing portfolio from another tracker or spreadsheet into
NetworthTracker with a dry run, clear mapping, validation, and an auditable
apply step.

## ADDED Requirements

### Requirement: Guided migration wizard
The system SHALL provide a guided migration flow that selects a source, maps
source fields to accounts, balances, securities, holdings, lots, crypto wallets,
and balances, and previews the result before writing.

#### Scenario: Mapping preview
- **WHEN** a user uploads a supported export and maps its fields
- **THEN** the system shows a dry-run summary of records to create, update, or
  reject before any write

### Requirement: Source adapters
Migration SHALL be implemented behind source adapters, starting with generic
CSV/JSON and allowing tracker-specific adapters without changing the core import
pipeline.

#### Scenario: Generic source
- **WHEN** a user selects the generic CSV/JSON source
- **THEN** the documented column mapping imports accounts, balances, and
  holdings/lots without requiring a specific third-party service

### Requirement: Validation and atomic apply
The migration SHALL validate every row and SHALL not write a partially valid
batch. Row-level errors SHALL identify the source row and reason.

#### Scenario: Invalid row
- **WHEN** any mapped row has an invalid amount, date, account, or security
- **THEN** the apply is rejected or explicitly partial according to the chosen
  mode, with row-level errors

### Requirement: Idempotent migration
Re-running the same migration SHALL update existing records using stable keys
rather than creating duplicates.

#### Scenario: Run twice
- **WHEN** the same export is migrated twice
- **THEN** accounts, balances, holdings, lots, and wallets are not duplicated

### Requirement: Provenance and local-only processing
Migrated records SHALL be tagged with their source and migration time, and all
processing SHALL occur locally without sending user data to a service.

#### Scenario: Source tag
- **WHEN** records are created by a migration
- **THEN** they retain a source/provenance marker and the migration timestamp

### Requirement: Pre-migration backup
The system SHALL create or require a verified backup before applying a data
migration, so the user can return to the pre-migration state.

#### Scenario: Migration backup
- **WHEN** a user confirms a migration
- **THEN** a verified backup is taken before writes begin
