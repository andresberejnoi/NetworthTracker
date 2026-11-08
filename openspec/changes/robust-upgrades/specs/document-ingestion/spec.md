# Spec Delta

## Purpose

Lets a user bring statements and transaction exports from different financial
institutions into NetworthTracker, while keeping the original sensitive files
locally, encrypted, tagged, auditable, and reusable for future tax/HSA work.

## ADDED Requirements

### Requirement: Local encrypted document store
The system SHALL store uploaded financial documents locally, encrypted at rest,
with content hashing, size limits, original filename, MIME type, uploader,
timestamps, and user-supplied tags. Original files SHALL be immutable and
downloadable only to authorized users.

#### Scenario: Upload and deduplicate
- **WHEN** a user uploads a file that is already stored
- **THEN** the system reuses the existing document by SHA-256 instead of storing
  a duplicate, while recording the new upload metadata

#### Scenario: Encrypted at rest
- **WHEN** a document is stored on disk
- **THEN** the file content is encrypted and the metadata records the
  encryption/key version, not a plaintext copy

### Requirement: File format detection
The system SHALL detect supported formats by content (magic bytes/text
sniffing) rather than trusting the file extension, and SHALL support CSV, XLSX,
XML/OFX/QFX, and PDF where a parser is available.

#### Scenario: Misleading extension
- **WHEN** a CSV file is named `.xlsx`
- **THEN** the system detects the actual format from its content and either
  parses it correctly or reports a clear unsupported/malformed error

#### Scenario: Unsupported file
- **WHEN** a file cannot be parsed
- **THEN** it is retained as an untagged document with an error status and is
  not imported

### Requirement: Institution and document-type classification
The system SHALL classify documents by institution (for example Fidelity,
Charles Schwab, Coinbase, Binance, or generic) and document type (transactions,
holdings, balances, statement, tax form, or other), with a confidence signal
and user confirmation before import.

#### Scenario: Known institution
- **WHEN** a Fidelity transaction CSV is uploaded
- **THEN** the system identifies the likely institution and type, shows the
  detected values and confidence, and asks the user to confirm before parsing

#### Scenario: Ambiguous document
- **WHEN** the system cannot identify the institution/type confidently
- **THEN** the user can choose the parser/type manually or leave it as an
  unclassified document

### Requirement: Preview, mapping, and idempotent apply
The system SHALL parse documents through source adapters, show a dry-run preview
with row-level errors, map records to accounts/securities/holdings/lots/wallets,
and apply atomically or explicitly partially with stable keys.

#### Scenario: Preview before write
- **WHEN** a supported document is parsed
- **THEN** the user sees what will be created/updated/rejected before any write

#### Scenario: Re-import
- **WHEN** the same document is imported twice
- **THEN** records are updated by stable keys rather than duplicated

### Requirement: Provenance and audit
Imported records SHALL link to the source document, institution, parser version,
and import time. Document access, download, classification, and import actions
SHALL be auditable.

#### Scenario: Auditing a balance
- **WHEN** a user inspects an imported balance
- **THEN** the source document and import event are visible

### Requirement: Cleanup and retention without deleting computed data
The system SHALL provide a cleanup command/script for orphaned or temporary
uploaded files with a dry-run and explicit confirmation. Cleanup MUST NOT delete
computed financial records; those are corrected or archived through audited
flows, never hard-deleted by the user.

#### Scenario: Orphan cleanup
- **WHEN** the cleanup command runs in dry-run mode
- **THEN** it lists only unreferenced/temporary files and changes nothing

#### Scenario: Computed data protection
- **WHEN** a user deletes a document
- **THEN** the original document may be soft-deleted/retained per policy, but
  imported balances, holdings, lots, and snapshots are not silently removed

### Requirement: Documents in backups
Database and document-store backups SHALL include uploaded files encrypted, and
restore SHALL preserve document metadata and provenance links.

#### Scenario: Restore documents
- **WHEN** a backup containing documents is restored
- **THEN** the encrypted files, metadata, tags, and provenance links are restored

## Future extension

Medical receipts/HSA reimbursement tracking is a separate future capability that
reuses the encrypted document store and audit trail; it is not implemented in
this change.
