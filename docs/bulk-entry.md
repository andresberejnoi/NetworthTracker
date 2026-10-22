# Bulk entry

Entering many records one form at a time is the wrong workflow. The **Bulk
entry** view is a spreadsheet-like grid: one row per record, keyboard-first
entry, and clipboard paste.

## Using it

1. Pick an account.
2. Type a date and amount, or paste a block copied from a spreadsheet.
3. Press **Enter** to add the next row (focus moves with you).
4. **Save balances** writes every valid row in one request and reports the
   result per row.

The grid accepts tab-separated blocks. Pasting twenty lines fills twenty rows in
one go — no per-record form.

## Validation

- Dates must be `YYYY-MM-DD`.
- Amounts must be decimal text (e.g. `1234.56`).
- Invalid cells are highlighted inline; saving is blocked until they are fixed.
- The server re-validates every row and reports row-level errors (for example a
  currency mismatch) with the row's original position.

## Batch endpoint

```bash
curl -s -X POST localhost:8080/api/v1/accounts/<id>/balances/batch \
  -H 'content-type: application/json' \
  -d '{
    "mode": "partial",
    "rows": [
      {"as_of": "2026-01-01", "amount": "10.00"},
      {"as_of": "2026-02-01", "amount": "20.00"}
    ]
  }'
```

```json
{
  "saved": [ { "as_of": "2026-01-01", "amount": "10.00", "...": "..." } ],
  "errors": [ { "index": 1, "error": "currency mismatch: ..." } ],
  "mode": "partial"
}
```

- `partial` — save the valid rows, report the rest.
- `atomic` (default) — reject the whole batch if any row is invalid.

Balances are upserted per account and day, so re-saving the same date replaces
that day's value rather than duplicating it.

## Notes

- The grid is a reusable component (`frontend/src/features/bulk-entry/BulkGrid.tsx`).
  It is wired to balances today; the same component is used for holdings and
  wallet addresses when those capabilities land (Groups 9 and 10).
- Rows are limited to 5000 per request.
