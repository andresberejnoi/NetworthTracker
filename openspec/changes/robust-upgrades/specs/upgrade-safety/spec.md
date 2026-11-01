# Spec Delta

## Purpose

Makes upgrades, backups, and releases safe enough for a household's real
financial data, with automatic protection before migrations and a tested path
back.

## ADDED Requirements

### Requirement: Automatic pre-migration backup
The container entrypoint SHALL create a verified backup before running any
database migration, and SHALL refuse to start if the backup cannot be created.

#### Scenario: Backup before migration
- **WHEN** a new image starts against an existing database
- **THEN** a verified encrypted backup is written before migrations run, and the
  backup path is logged

#### Scenario: Backup failure blocks startup
- **WHEN** the pre-migration backup cannot be written or verified
- **THEN** the application does not run migrations and exits with a clear error

### Requirement: Scheduled and off-site backups
The system SHALL support user-configured scheduled backups, retention, and an
off-site destination, with encryption enabled by default and the encryption key
kept separate from the backup files.

#### Scenario: Scheduled backup
- **WHEN** the user enables a schedule
- **THEN** encrypted backups are produced on that schedule and old backups are
  pruned according to retention

#### Scenario: Off-site copy
- **WHEN** the user configures a destination
- **THEN** backups are copied there directly from the user's machine without a
  NetworthTracker-operated relay

#### Scenario: Missing key
- **WHEN** no backup encryption key is configured
- **THEN** scheduled backups fail closed rather than writing plaintext

### Requirement: Restore verification
Every backup path SHALL be restorable, and the release process SHALL include a
restore drill that verifies data after decryption and import.

#### Scenario: Restore drill
- **WHEN** a release candidate is verified
- **THEN** a backup is restored into a throwaway database and key rows and exact
  decimal values are checked

### Requirement: Versioned artifacts
Release images SHALL be published with the product version and the Compose
configuration SHALL support pinning a version rather than relying on `latest`.

#### Scenario: Pinned release
- **WHEN** a user deploys a release
- **THEN** the backend/frontend images carry the same `v<version>` tag and the
  running API reports that version

### Requirement: Upgrade compatibility policy
Migrations SHALL be forward-only and documented as expand/contract; the project
SHALL state which previous releases can upgrade directly and how unknown
migrations are handled.

#### Scenario: N-1 upgrade
- **WHEN** a database from the previous supported release is upgraded
- **THEN** the current release migrates it and preserves existing data

#### Scenario: Unknown migration
- **WHEN** the database contains a migration the binary does not know about
- **THEN** startup is refused instead of running an older binary against a newer
  schema

### Requirement: Multi-release upgrade matrix
CI SHALL upgrade fixture databases from supported prior releases to the current
release and verify data and the reported version.

#### Scenario: Matrix run
- **WHEN** a release is prepared
- **THEN** each supported prior release is upgraded with fixture data and checked
  for data preservation and the correct reported version
