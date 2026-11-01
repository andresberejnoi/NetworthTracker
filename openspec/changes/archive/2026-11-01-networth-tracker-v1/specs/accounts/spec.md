# Spec Delta

## Purpose

Maintains the registry of financial accounts and their balances across every supported account type,
forming the single source of truth for what the household owns and owes.

## ADDED Requirements

### Requirement: Supported account types
The system SHALL support at least these account types: 401(k), Traditional IRA, Roth IRA, taxable
brokerage, checking, savings, cash, real estate, other asset, and liability accounts such as
mortgages or loans.

#### Scenario: Creating a retirement account
- **WHEN** a user creates an account and selects the 401(k) type
- **THEN** the account is stored as a retirement asset and is available for net worth aggregation

#### Scenario: Creating a liability account
- **WHEN** a user records a mortgage as a liability account
- **THEN** the system treats its balance as an amount owed rather than an amount owned

### Requirement: Asset and liability classification
Every account type SHALL be classified as either an asset or a liability so aggregation and net
worth math can be applied consistently.

#### Scenario: Classification applied in aggregation
- **WHEN** net worth is computed for a household
- **THEN** asset account balances are added and liability account balances are subtracted

### Requirement: Account identity and metadata
Each account SHALL have a name, an account type, a currency, an ownership scope, and an optional
institution or custodian; any stored account number MUST be masked so the full number is not
recoverable from the application.

#### Scenario: Masked account number
- **WHEN** a user enters an account number while creating an account
- **THEN** the system stores only a masked form and never displays the full number

### Requirement: Dated balance records
The system SHALL store dated balance records for each account so historical net worth can be
reconstructed entirely from local data.

#### Scenario: Multiple balances over time
- **WHEN** a user records a balance for the same account on two different dates
- **THEN** both dated records are retained and the account's value on each date is recoverable

#### Scenario: Historical reconstruction offline
- **WHEN** the machine has no network connection
- **THEN** the system can still display the account's balance history from stored records

### Requirement: Fully offline manual entry
The system SHALL allow a user to create accounts and record balances without any network connection.

#### Scenario: Creating an account offline
- **WHEN** a user creates an account and records its balance while disconnected
- **THEN** the account and balance are saved locally and included in net worth immediately

### Requirement: Account archival preserves history
The system SHALL allow an account to be archived or closed without deleting its historical balance
records.

#### Scenario: Archiving a closed account
- **WHEN** a user archives an account
- **THEN** the account stops appearing in current balances but its past history remains available for charts
