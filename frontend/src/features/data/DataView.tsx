import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  exportUrl,
  importSpreadsheet,
  templateUrl,
  type ImportReport,
} from "../../api/client";

export default function DataView() {
  const queryClient = useQueryClient();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const handleFile = async (file: File | undefined) => {
    if (!file) return;

    setBusy(true);
    setError(null);
    setReport(null);

    try {
      const result = await importSpreadsheet(file);
      setReport(result);
      if (result.errors.length === 0) {
        queryClient.invalidateQueries({ queryKey: ["accounts"] });
        queryClient.invalidateQueries({ queryKey: ["networth"] });
      }
    } catch (importError) {
      setError((importError as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card" aria-label="data import and export">
      <h2>Import</h2>
      <p className="muted">
        Import accounts and balances from CSV or XLSX. The file is validated in
        full first — if any row is invalid, nothing is written.
      </p>

      <label className="field">
        <span>Spreadsheet</span>
        <input
          aria-label="spreadsheet file"
          type="file"
          accept=".csv,.xlsx"
          disabled={busy || !canWrite}
          onChange={(event) => handleFile(event.target.files?.[0])}
        />
      </label>

      {!canWrite && (
        <p className="muted">Your role is read-only, so imports are disabled.</p>
      )}

      {busy && <p className="muted">Importing …</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}

      {report && (
        <div className="bulk-result" role="status">
          <p>
            Created {report.accounts_created} account
            {report.accounts_created === 1 ? "" : "s"}, saved{" "}
            {report.balances_saved} balance
            {report.balances_saved === 1 ? "" : "s"}.
          </p>
          {report.errors.length > 0 && (
            <>
              <p>Nothing was written. Fix these rows and try again:</p>
              <ul>
                {report.errors.map((rowError) => (
                  <li key={rowError.row}>
                    Line {rowError.row}: {rowError.error}
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}

      <h2>Export</h2>
      <p className="muted">
        Download every account and balance. Re-importing the file is idempotent.
      </p>
      <div className="bulk-actions">
        <a className="button-link" href={exportUrl("csv")}>
          Export CSV
        </a>
        <a className="button-link" href={exportUrl("xlsx")}>
          Export XLSX
        </a>
      </div>

      <h2>Template</h2>
      <p className="muted">Start from the versioned template.</p>
      <div className="bulk-actions">
        <a className="button-link" href={templateUrl("csv")}>
          Template CSV
        </a>
        <a className="button-link" href={templateUrl("xlsx")}>
          Template XLSX
        </a>
      </div>

      <h2>Samples</h2>
      <p className="muted">
        The repository includes sample spreadsheets with different account types
        under <code>samples/</code>.
      </p>
    </section>
  );
}
