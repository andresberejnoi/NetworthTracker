# Spec Delta

## Purpose

Extends the portable JSON backup/restore format so a household can move all of
its v1 data, including investments and crypto, between machines without relying
only on a PostgreSQL dump.

## ADDED Requirements

### Requirement: Complete investment and crypto backup coverage
The portable JSON backup SHALL include securities, holdings, tax lots, sales and
disposals, crypto assets, wallets, balances, budgets, and aggregation
provenance, and SHALL restore them with stable keys.

#### Scenario: Backup and restore investments
- **WHEN** a user exports and restores a household with securities, lots, and
  sales
- **THEN** the positions, remaining lots, disposals, and realized gains are
  restored without duplication

#### Scenario: Backup and restore crypto
- **WHEN** a user exports and restores crypto wallets, balances, and lots
- **THEN** the wallets, exact quantities, lots, and realized gains are restored

#### Scenario: Budgets and provenance
- **WHEN** a household has budgets and aggregation provenance
- **THEN** the backup includes them and restore preserves their source/time tags

### Requirement: Backward-compatible backup versions
The backup format SHALL be versioned, SHALL restore v1 backups, and SHALL reject
unknown future versions with a clear error.

#### Scenario: Restore a v1 backup
- **WHEN** a v1 backup is restored by the current release
- **THEN** accounts, balances, and snapshots are restored and the newer sections
  default to empty

#### Scenario: Future version
- **WHEN** a backup declares an unsupported newer format version
- **THEN** restore refuses rather than silently dropping data

### Requirement: Secrets excluded or encrypted
Portable JSON backups SHALL NOT contain plaintext provider tokens, passwords, or
session material; aggregation credentials remain in the encrypted local store.

#### Scenario: No plaintext token
- **WHEN** a JSON backup is inspected
- **THEN** no provider token or password appears in plaintext
