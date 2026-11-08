# Design

## Decisions

### D1. Backup before migrations in the entrypoint, fail closed

**Decision:** The container entrypoint compares the database's applied migration
set with the embedded migration set. If migrations are pending, it creates and
verifies an encrypted backup before starting the backend. If the backup cannot
be created, startup stops. The existing `make upgrade` backup remains as an
operator-level safety net.

**Rationale:** A host-side script can be bypassed by `docker compose up` or a
restart after a pull. The entrypoint is the last point that can guarantee a
backup before schema changes. Backing up only when migrations are pending avoids
a full dump on every start.

**Alternatives considered:** Always backing up on every start (rejected: slow
and noisy); relying only on `scripts/upgrade.sh` (rejected: bypassable).

### D2. Scheduled backups are host-scheduled, user-destined, and key-separated

**Decision:** Ship a backup command suitable for cron/systemd timers rather than
a long-running backup daemon. The user configures the schedule, retention, and an
optional off-site destination (for example an S3-compatible bucket or rclone
remote). Encryption uses the existing GPG AES-256 path; the passphrase/key must
not live in the backup directory. A restore-drill command verifies the newest
backup against a throwaway database.

**Rationale:** Local-first products should not operate a backup relay. A host
timer is simple, inspectable, and works with the user's own storage. Separating
the key is the difference between an encrypted backup and a locked box with the
key taped to it.

**Alternatives considered:** A built-in scheduler daemon (rejected: more moving
parts and container lifecycle complexity); cloud backup included by the product
(rejected: privacy, cost, and central-service constraints).

### D3. Versioned images and pinned Compose deployments

**Decision:** Publish backend and frontend images tagged with the product
version, plus an optional `latest` convenience tag. Compose supports an explicit
image tag/version and release docs recommend pinning it. The running API version
must match the tag.

**Rationale:** Rebuilding from source makes rollback and support ambiguous. A
versioned image is the unit a user can pin, inspect, and restore against.

### D4. Forward-only migrations with an expand/contract policy

**Decision:** Migrations remain forward-only. Breaking schema changes use
expand/contract across two releases: add the new shape and dual-write/backfill,
then remove the old shape later. The current release supports direct upgrades
from the previous release (N-1); older databases must step through an intermediate
release. SQLx's unknown-migration guard remains mandatory.

**Rationale:** This gives a concrete compatibility promise without pretending
downgrades are safe. Expand/contract keeps old and new application versions able
to coexist during a rolling upgrade.

### D5. Multi-release upgrade matrix in CI

**Decision:** CI builds the current release and each supported prior release,
starts a database from the prior release with fixture data, runs the current
image, and asserts data preservation and the reported version. The matrix is
generated from release tags rather than hand-maintained.

**Rationale:** The existing migration test covers the last migrations only. A
tag-to-tag matrix catches real upgrade paths, including entrypoint backup and
version reporting.

### D6. Portable JSON backup v2, backward-compatible

**Decision:** Extend the JSON backup with investments, tax lots, sales/disposals,
crypto assets/wallets/balances, budgets, and aggregation provenance. Keep stable
keys (account name, security symbol, wallet identity, period/category) for
idempotent restore. Read v1 backups by defaulting missing sections to empty;
reject unknown future versions. Do not include plaintext provider tokens,
passwords, or sessions. The full `pg_dump` backup remains the authoritative
disaster-recovery path.

**Rationale:** The portable backup is how users move data between machines; it
must cover the data users actually created. Stable keys and versioning prevent
duplication and silent data loss.

### D8. Encrypted, content-addressed document store

**Decision:** Store uploaded files under a content-addressed local store,
encrypted with a local key (AES-256-GCM or the existing GPG mechanism), with
metadata in PostgreSQL: SHA-256, original name, MIME, size, uploader, tags,
institution, document type, period, parser version, status, and audit events.
Deduplicate by SHA-256 and keep the original immutable; imported records link to
the document.

**Rationale:** Statements are both sensitive and the audit trail for imported
numbers. A separate encrypted store keeps them out of the database dump while
still linking them to computed data.

### D9. Detect content first, then classify with confirmation

**Decision:** Detect formats by magic bytes/text sniffing and parser adapters,
not extension alone. Institution/type classification uses parser-specific
heuristics with a confidence score; the user confirms or overrides before
import. PDF parsing is best-effort text/table extraction; unsupported files are
retained as unclassified documents rather than silently dropped.

**Rationale:** Institution exports vary and extensions lie. A wrong parser can
corrupt financial data; confirmation and preview make classification explicit.

### D10. Sensitive-file lifecycle and cleanup

**Decision:** Original documents are immutable and soft-deletable; a cleanup
command removes only unreferenced/temporary files after a dry run and explicit
confirmation. Imported balances, holdings, lots, and snapshots are never
hard-deleted through the document UI; corrections use audited adjustments or
archival. Document-store backups include encrypted files and metadata.

**Rationale:** Files are evidence; computed data is the ledger. Cleanup should
reclaim storage, not rewrite history.

### D11. HSA/medical receipts are a separate future capability

**Decision:** The document store and audit trail are reusable by a future
`medical-receipts`/HSA capability. That capability owns receipt categorization,
HSA eligibility, reimbursement tracking, and tax-year reporting; it is not part
of robust-upgrades.

**Rationale:** HSA receipt tracking is a distinct domain with its own rules and
privacy requirements. Keeping it separate avoids overloading the upgrade/migration
change.

### D7. Guided migration behind source adapters

**Decision:** Build a migration pipeline with source adapters (generic CSV/JSON
first, tracker-specific later), field mapping, validation, a dry-run preview,
atomic apply (or explicit partial mode), idempotent stable keys, and provenance
tags. The pipeline runs locally, requires a verified backup before apply, and
never calls a central service.

**Rationale:** Importing a real portfolio is a one-time, high-stakes operation.
A dry run and row-level errors are more valuable than a one-click importer that
silently mangles lots or duplicates accounts.

## Risks / Trade-offs

- **Entrypoint backups add startup time when migrations are pending** → only run
  when the applied/embedded migration sets differ, and stream to disk.
- **Scheduled off-site copies can expose data** → encrypt before copying, keep
  the key separate, and document destination permissions.
- **Multi-release CI grows with supported versions** → support only N-1 directly
  and generate the matrix from tags.
- **JSON backup v2 grows the surface** → keep it additive and idempotent; the
  full database dump remains the primary restore path.
- **Migration adapters can mis-map lots** → dry-run, row-level errors, stable
  keys, provenance, and a pre-apply backup.
