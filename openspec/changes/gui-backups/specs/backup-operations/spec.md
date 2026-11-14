# Spec Delta

## Purpose

Defines owner-controlled GUI backup creation and encrypted backup download
without exposing host shell access to the browser.

## ADDED Requirements

### Requirement: Owner-only GUI backup creation
The system SHALL let an owner create a full encrypted database backup from the
Settings → Backups page, including the encrypted document archive when present.

#### Scenario: Create backup
- **WHEN** an owner selects Back up now
- **THEN** the backend creates a verified PostgreSQL dump and optional document
  archive under the configured backup directory, applies local retention, and
  returns backup metadata to the UI

#### Scenario: Non-owner
- **WHEN** a non-owner calls the backup endpoints
- **THEN** the request is rejected with a forbidden response

### Requirement: Fail-closed encryption
The system SHALL refuse to create a GUI backup when the backup passphrase is
missing, blank, or a placeholder, and SHALL NOT write a plaintext dump.

#### Scenario: Missing passphrase
- **WHEN** an owner requests a backup without a configured passphrase
- **THEN** no backup is created and the UI reports a configuration error

#### Scenario: Verification failure
- **WHEN** the encrypted archive cannot be verified
- **THEN** the partial file is removed and the operation reports failure

### Requirement: Backup listing and encrypted download
The system SHALL list known backup files without exposing arbitrary paths and
SHALL let an owner download an encrypted backup file.

#### Scenario: List backups
- **WHEN** an owner opens Settings → Backups
- **THEN** recent database and document archives are shown with timestamps and
  sizes, newest first

#### Scenario: Download
- **WHEN** an owner selects a listed backup
- **THEN** the encrypted file is downloaded through the authenticated API

#### Scenario: Invalid filename
- **WHEN** a request contains a path traversal or a non-backup filename
- **THEN** the request is rejected without reading outside the backup directory

### Requirement: Single backup operation
The system SHALL prevent overlapping GUI backup runs.

#### Scenario: Concurrent request
- **WHEN** a backup is already running and another create request arrives
- **THEN** the second request is rejected with a conflict/busy response

### Requirement: Host scheduling remains available
The system SHALL keep the host cron/systemd backup workflow and SHALL NOT
require it for a manual GUI backup.

#### Scenario: Existing timer
- **WHEN** the host timer runs while GUI backups are available
- **THEN** the existing encrypted scheduled backup workflow continues to work

### Requirement: No GUI restore in this change
The system SHALL NOT expose full database restore from the GUI; restore remains
the host/CLI workflow until a supervised maintenance flow exists.

#### Scenario: Restore attempt
- **WHEN** a user looks for restore controls
- **THEN** the UI explains that restore is intentionally host-only for now
