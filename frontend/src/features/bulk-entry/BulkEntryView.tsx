import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listAccounts, recordBalancesBatch } from "../../api/client";
import BulkGrid, {
  type GridColumn,
  type GridRow,
  type SaveResult,
} from "./BulkGrid";

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;
const DECIMAL = /^-?\d+(\.\d+)?$/;

const COLUMNS: GridColumn[] = [
  {
    key: "as_of",
    label: "Date",
    placeholder: "YYYY-MM-DD",
    validate: (value) =>
      value === ""
        ? "date is required"
        : ISO_DATE.test(value)
          ? null
          : "use YYYY-MM-DD",
  },
  {
    key: "amount",
    label: "Amount",
    placeholder: "0.00",
    validate: (value) =>
      value === ""
        ? "amount is required"
        : DECIMAL.test(value)
          ? null
          : "enter a decimal amount",
  },
];

export default function BulkEntryView() {
  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => listAccounts(),
  });
  const [accountId, setAccountId] = useState("");

  const selected = accountId || accounts.data?.[0]?.id || "";

  const handleSave = async (rows: GridRow[]): Promise<SaveResult> => {
    if (!selected) {
      throw new Error("Select an account first");
    }

    const result = await recordBalancesBatch(
      selected,
      rows.map((row) => ({ as_of: row.as_of, amount: row.amount })),
      "partial",
    );

    return {
      savedCount: result.saved.length,
      errors: result.errors,
    };
  };

  return (
    <section className="card" aria-label="bulk entry">
      <h2>Bulk entry</h2>
      <p className="muted">
        Paste a block of rows from a spreadsheet, or type row after row. Press
        Enter to add the next row.
      </p>

      {accounts.isPending && <p className="muted">Loading accounts …</p>}
      {accounts.isError && (
        <p className="error" role="alert">
          {(accounts.error as Error).message}
        </p>
      )}

      {accounts.data && accounts.data.length === 0 && (
        <p className="muted">
          Create an account first, then come back to paste balances.
        </p>
      )}

      {accounts.data && accounts.data.length > 0 && (
        <>
          <label className="field">
            <span>Account</span>
            <select
              aria-label="bulk account"
              value={selected}
              onChange={(event) => setAccountId(event.target.value)}
            >
              {accounts.data.map((account) => (
                <option key={account.id} value={account.id}>
                  {account.name}
                </option>
              ))}
            </select>
          </label>

          <BulkGrid
            columns={COLUMNS}
            minRows={1}
            onSave={handleSave}
            saveLabel="Save balances"
          />
        </>
      )}
    </section>
  );
}
