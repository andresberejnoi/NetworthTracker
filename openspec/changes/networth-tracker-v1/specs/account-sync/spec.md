# Spec Delta

## Purpose

Defines a provider-agnostic, read-only interface for pulling balances and holdings from external
institutions, so real account linking can be added later without changing the domain model.

## ADDED Requirements

### Requirement: Read-only aggregation adapter interface
The system SHALL define a provider-agnostic adapter interface for connecting external accounts, and
every adapter MUST be read-only: it MUST NOT initiate transfers, trades, payments, or any state
change at the provider.

#### Scenario: Interface independence
- **WHEN** a new aggregation provider is added
- **THEN** it is implemented behind the existing adapter interface without changes to the account, holdings, or net worth domain behavior

#### Scenario: No write operations
- **WHEN** an adapter is exercised
- **THEN** it is capable only of retrieving account, balance, and holding data

### Requirement: Imported data mapped with provenance
Data imported from an external provider SHALL be mapped onto accounts and holdings and tagged with
its source and the time of synchronization.

#### Scenario: Synced balance tagged
- **WHEN** a provider returns a balance for a linked account
- **THEN** the stored balance records that it came from that provider along with the sync time

### Requirement: Optional and non-blocking
The core product SHALL remain fully usable when no aggregation provider is configured, and a failed
or unreachable provider MUST NOT prevent account management or net worth computation.

#### Scenario: Unconfigured provider
- **WHEN** no aggregation provider is configured
- **THEN** all account, holdings, and net worth features continue to work using manual and imported data

#### Scenario: Provider outage
- **WHEN** a configured provider cannot be reached during a sync
- **THEN** the sync fails gracefully, existing data is retained, and the user is informed

### Requirement: Local credential handling
Provider credentials, tokens, and session material SHALL be stored only on the local machine and
MUST NOT be transmitted anywhere other than to the provider they belong to.

#### Scenario: Tokens stay local
- **WHEN** an adapter authenticates with a provider
- **THEN** the resulting tokens are stored on the local machine and are not sent to any third-party service

### Requirement: Provenance-preserving updates
WHEN a provider supplies a value for an account that also has manual entries, the system SHALL
preserve provenance and MUST NOT silently discard the manual value.

#### Scenario: Manual value retained
- **WHEN** a provider sync updates an account that has a manually entered balance for the same date
- **THEN** both values and their sources are retained and the user is shown which value is in effect

### Requirement: User-supplied provider subscription
The system SHALL require the individual user to obtain and configure their own aggregation provider
subscription or credentials, and MUST NOT operate a central aggregation service or bundle credentials
on the user's behalf.

#### Scenario: User's own subscription
- **WHEN** a user enables an aggregation add-on
- **THEN** they connect using their own provider account, API key, or subscription

#### Scenario: No intermediary service
- **WHEN** an adapter retrieves data from a provider
- **THEN** data flows between the user's machine and the provider without an intermediary operated by the product
