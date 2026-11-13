import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import {
  applyMigration,
  previewMigration,
  type MigrationPreview,
  type MigrationReport,
  type MigrationSourceKind,
} from "../../api/client";
import EmptyState from "../../components/EmptyState";
import FormField from "../../components/FormField";
import PageHeader from "../../components/PageHeader";

async function readText(file: File): Promise<string> {
  if (typeof file.text === "function") return file.text();
  return await new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error ?? new Error("could not read file"));
    reader.readAsText(file);
  });
}

export default function MigrationView() {
  const [file, setFile] = useState<File | null>(null);
  const [content, setContent] = useState("");
  const [source, setSource] = useState<MigrationSourceKind>("generic_json");
  const [mapping, setMapping] = useState("{}");
  const [mode, setMode] = useState<"atomic" | "partial">("atomic");
  const [backupConfirmed, setBackupConfirmed] = useState(false);
  const [preview, setPreview] = useState<MigrationPreview | null>(null);
  const [report, setReport] = useState<MigrationReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadFile = async (next: File | undefined) => {
    setPreview(null);
    setReport(null);
    setError(null);
    if (!next) {
      setFile(null);
      setContent("");
      return;
    }
    setFile(next);
    setSource(next.name.toLowerCase().endsWith(".json") ? "generic_json" : "generic_csv");
    try {
      setContent(await readText(next));
    } catch (readError) {
      setError((readError as Error).message);
    }
  };

  const parseMapping = (): Record<string, string> => {
    const parsed = JSON.parse(mapping) as unknown;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("Mapping must be a JSON object.");
    }
    return parsed as Record<string, string>;
  };

  const runPreview = useMutation({
    mutationFn: () =>
      previewMigration({ source, mapping: parseMapping(), content }),
    onSuccess: setPreview,
    onError: (previewError) => setError((previewError as Error).message),
  });

  const runApply = useMutation({
    mutationFn: () =>
      applyMigration({
        source,
        mapping: parseMapping(),
        content,
        mode,
        backup_confirmed: true,
      }),
    onSuccess: setReport,
    onError: (applyError) => setError((applyError as Error).message),
  });

  return (
    <section>
      <PageHeader
        title="Migration"
        description="Preview and import a portfolio from a generic CSV or JSON export."
      />

      <div className="card">
        <FormField
          label="Source file"
          hint="CSV with a header row, or a JSON array of objects."
        >
          <input
            aria-label="migration file"
            type="file"
            accept=".csv,.json"
            onChange={(event) => void loadFile(event.target.files?.[0])}
          />
        </FormField>

        <div className="grid-two">
          <FormField label="Source format">
            <select
              aria-label="migration source"
              value={source}
              onChange={(event) => setSource(event.target.value as MigrationSourceKind)}
            >
              <option value="generic_json">Generic JSON</option>
              <option value="generic_csv">Generic CSV</option>
            </select>
          </FormField>
          <FormField label="Mode">
            <select
              aria-label="migration mode"
              value={mode}
              onChange={(event) => setMode(event.target.value as "atomic" | "partial")}
            >
              <option value="atomic">Atomic — reject any invalid row</option>
              <option value="partial">Partial — write valid rows</option>
            </select>
          </FormField>
        </div>

        <FormField
          label="Field mapping (JSON)"
          hint='Optional. Example: {"account_name": "Account", "amount": "Balance"}'
        >
          <textarea
            aria-label="migration mapping"
            className="code-input"
            value={mapping}
            onChange={(event) => setMapping(event.target.value)}
          />
        </FormField>

        <div className="bulk-actions">
          <button
            type="button"
            disabled={!file || content === "" || runPreview.isPending}
            onClick={() => {
              setError(null);
              setReport(null);
              try {
                parseMapping();
              } catch (mappingError) {
                setError((mappingError as Error).message);
                return;
              }
              runPreview.mutate();
            }}
          >
            {runPreview.isPending ? "Previewing…" : "Preview"}
          </button>
        </div>

        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}

        {preview && (
          <div className="preview-panel" role="status">
            <h3>Preview</h3>
            <p className="muted">
              {preview.total_rows} source row(s): {preview.accounts} account(s),{" "}
              {preview.balances} balance(s), {preview.holding_lots} holding lot(s),{" "}
              {preview.crypto_wallets} crypto wallet(s),{" "}
              {preview.crypto_balances} crypto balance(s),{" "}
              {preview.crypto_lots} crypto lot(s).
            </p>
            {preview.errors.length > 0 && (
              <>
                <p className="error">Rows that would be rejected:</p>
                <ul>
                  {preview.errors.map((rowError) => (
                    <li key={`${rowError.row}-${rowError.message}`}>
                      Row {rowError.row}: {rowError.message}
                    </li>
                  ))}
                </ul>
              </>
            )}
            {preview.errors.length === 0 && (
              <p className="notice">No validation errors.</p>
            )}
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={backupConfirmed}
                onChange={(event) => setBackupConfirmed(event.target.checked)}
              />
              I took a verified backup with <code>make backup</code>
            </label>
            <button
              type="button"
              disabled={
                !backupConfirmed ||
                runApply.isPending ||
                (preview.errors.length > 0 && mode === "atomic")
              }
              onClick={() => {
                setError(null);
                runApply.mutate();
              }}
            >
              {runApply.isPending ? "Applying…" : "Apply migration"}
            </button>
          </div>
        )}

        {report && (
          <div className="preview-panel" role="status">
            <h3>Migration applied</h3>
            <p className="muted">
              {report.accounts_created} account(s), {report.balances_saved} balance(s),{" "}
              {report.holding_lots_saved} holding lot(s),{" "}
              {report.crypto_lots_saved} crypto lot(s). Migration{" "}
              <code>{report.migration_id}</code>.
            </p>
            {report.errors.length > 0 && (
              <ul>
                {report.errors.map((rowError) => (
                  <li key={`${rowError.row}-${rowError.message}`}>
                    Row {rowError.row}: {rowError.message}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>

      <EmptyState
        title="Need a repeatable CLI flow?"
        description="The migration CLI takes the verified backup automatically and supports the same preview/apply contract."
        action={
          <code>./scripts/migrate.sh --file samples/migration-portfolio.json --apply</code>
        }
      />
    </section>
  );
}
