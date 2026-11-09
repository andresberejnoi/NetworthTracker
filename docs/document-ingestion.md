# Document ingestion

Uploaded statements and exports stay on the local machine. The original file is
stored encrypted at rest, content-addressed by SHA-256, and linked to metadata,
tags, classification, audit events, and any imported records through the
migration provenance.

This is not a cloud document service. There is no central relay.

## Configuration

```env
DOCUMENT_ENCRYPTION_KEY=<base64 32-byte key>
DOCUMENT_DIR=./documents
DOCUMENT_MAX_BYTES=20971520
```

Generate the key once:

```bash
python3 -c 'import base64,os; print(base64.b64encode(os.urandom(32)).decode())'
```

Keep the key separate from the `documents/` directory and from backups. Losing
it makes the stored files unreadable. Compose mounts `./documents` at
`/documents` and passes the same key to the backend. Without a valid key the API
starts normally but document uploads are rejected with a configuration error.

## Key rotation and recovery

The document key is separate from `BACKUP_PASSPHRASE` and is **not** included in
backups. Keep at least one copy in a password manager or on another machine.

This release does not include an automated document-key rotation command. Do not
rotate by only changing `DOCUMENT_ENCRYPTION_KEY` in `.env`: existing files would
become unreadable because their nonces/key version are tied to the old key.
Until a rotation command is added, preserve the old key for as long as old
documents are needed, or plan a maintenance procedure that decrypts and
re-encrypts every `*.enc` file and updates its `documents.nonce`/`key_version`
metadata. Test that procedure on a copy and keep a verified backup first.

## Upload and storage

- Upload from the **Documents** view (owner only) or
  `POST /api/v1/documents?filename=...&tags=...` with the raw file bytes.
- Files are encrypted with AES-256-GCM before being written.
- The file name on disk is the SHA-256 plaintext hash plus `.enc`; duplicate
  content reuses the file and records a deduplicated audit event.
- The original is immutable. Soft-delete removes it from normal listings but
  leaves the encrypted file and metadata for audit/cleanup policy.
- Size limit: `DOCUMENT_MAX_BYTES` (20 MB default).

## Detection, parsing, and classification

The extension is not trusted. Detection uses magic bytes and text sniffing:

| Format | Detection/parser | Behavior |
|---|---|---|
| CSV | text/header sniff | full table preview |
| XLSX | ZIP magic + calamine | first worksheet preview |
| XML | `<...>` text | leaf tag/value preview |
| OFX/QFX | `<OFX>` marker | tag/value/transaction preview |
| PDF | `%PDF` magic | best-effort text extraction; scanned PDFs are retained unparsed |

Institution heuristics recognize Fidelity, Charles Schwab, Coinbase, Binance, and
generic sources; document types include transactions, holdings, balances,
statement, tax form, and other. The detected values and confidence are stored,
shown in the preview, and can be confirmed or overridden in the UI/API.

Unsupported or unparsable files are still retained as `unparsed` documents;
they are never silently dropped and are never imported automatically.

## Preview, mapping, and import

`POST /api/v1/documents/{id}/preview` returns the detected classification, parsed
table, audit events, and any parse error. The UI shows the table and lets the
owner enter a field mapping.

`POST /api/v1/documents/{id}/import` accepts:

```json
{
  "mapping": { "account_name": "Account", "amount": "Balance" },
  "mode": "atomic",
  "backup_confirmed": true
}
```

It converts the parsed table to the generic migration CSV adapter and runs the
same previewed/apply pipeline used by `scripts/migrate.sh`. Take a verified
backup first; `backup_confirmed` is a safety gate. The response includes the
`migration_id`, and the document is linked to that migration run through
`document_provenance`. The migration provenance rows link the run to the written
accounts, holdings, balances, and crypto records.

## Audit

Upload, deduplicated upload, download, classification/tag changes, import, and
soft-delete actions are written to `document_audit_events`. `GET
/api/v1/documents/{id}/audit` returns the history. Audit metadata is JSON; no
plaintext file content is stored in the audit table.

## Backups and restore

`make backup` includes the encrypted `documents/` directory in a separate
`networth-documents-<timestamp>.tar.gz.gpg` archive (when the directory exists).
The archive is verified before it is kept. `make restore` restores the matching
document archive alongside the database dump if it is present. Database metadata
and provenance links therefore survive a restore; the files remain encrypted
throughout. Off-site rclone copies include the document archive too.

## Cleanup

```bash
make documents-cleanup        # dry run: list only
make documents-cleanup-apply  # typed confirmation required
```

The cleanup script reads the authoritative `documents.sha256` set from the
database and only considers files in `DOCUMENT_DIR`. It removes unreferenced
`*.enc` files and temporary `*.tmp` files after confirmation. It never deletes
database rows or computed balances, holdings, lots, snapshots, or budgets. A
database read failure aborts the cleanup.

## Future HSA/medical receipts

The encrypted store, tags, audit trail, and provenance links are intentionally
reusable. HSA/medical-receipt tracking is a separate future capability that will
own receipt categorization, eligibility, reimbursement, and tax-year reporting;
it is not part of this change.
