import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteDocument,
  documentAudit,
  downloadDocument,
  importDocument,
  listDocuments,
  previewDocument,
  updateDocument,
  uploadDocument,
  type DocumentAuditEvent,
  type DocumentMetadata,
  type DocumentPreview,
  type MigrationReport,
} from "../../api/client";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function DocumentsView() {
  const queryClient = useQueryClient();
  const [file, setFile] = useState<File | null>(null);
  const [tags, setTags] = useState("statement");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [selected, setSelected] = useState<DocumentMetadata | null>(null);
  const [preview, setPreview] = useState<DocumentPreview | null>(null);
  const [audit, setAudit] = useState<DocumentAuditEvent[]>([]);
  const [mapping, setMapping] = useState("{}");
  const [mode, setMode] = useState<"atomic" | "partial">("atomic");
  const [backupConfirmed, setBackupConfirmed] = useState(false);
  const [report, setReport] = useState<MigrationReport | null>(null);

  const documents = useQuery({
    queryKey: ["documents"],
    queryFn: () => listDocuments(true),
  });

  const handleUpload = async () => {
    if (!file) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const document = await uploadDocument(
        file,
        tags
          .split(",")
          .map((tag) => tag.trim())
          .filter(Boolean),
      );
      setFile(null);
      setNotice(`Stored ${document.original_name} (${document.sha256.slice(0, 12)}…)`);
      await queryClient.invalidateQueries({ queryKey: ["documents"] });
    } catch (uploadError) {
      setError(errorMessage(uploadError));
    } finally {
      setBusy(false);
    }
  };

  const openDocument = async (document: DocumentMetadata) => {
    setSelected(document);
    setPreview(null);
    setAudit([]);
    setReport(null);
    setError(null);
    try {
      const [parsed, events] = await Promise.all([
        previewDocument(document.id),
        documentAudit(document.id),
      ]);
      setPreview(parsed);
      setAudit(events);
    } catch (previewError) {
      setError(errorMessage(previewError));
    }
  };

  const handleDownload = async (document: DocumentMetadata) => {
    try {
      const blob = await downloadDocument(document.id);
      const url = URL.createObjectURL(blob);
      const anchor = window.document.createElement("a");
      anchor.href = url;
      anchor.download = document.original_name;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (downloadError) {
      setError(errorMessage(downloadError));
    }
  };

  const handleTags = async (document: DocumentMetadata) => {
    const next = window.prompt("Tags (comma separated)", document.tags.join(", "));
    if (next === null) return;
    try {
      const updated = await updateDocument(document.id, {
        tags: next
          .split(",")
          .map((tag) => tag.trim())
          .filter(Boolean),
      });
      setSelected((current) => (current?.id === updated.id ? updated : current));
      await queryClient.invalidateQueries({ queryKey: ["documents"] });
    } catch (tagError) {
      setError(errorMessage(tagError));
    }
  };

  const handleDelete = async (document: DocumentMetadata) => {
    if (!window.confirm(`Soft-delete ${document.original_name}?`)) return;
    try {
      await deleteDocument(document.id);
      setSelected(null);
      setPreview(null);
      await queryClient.invalidateQueries({ queryKey: ["documents"] });
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    }
  };

  const handleImport = async () => {
    if (!selected || !backupConfirmed) return;
    let parsedMapping: Record<string, string>;
    try {
      parsedMapping = JSON.parse(mapping) as Record<string, string>;
    } catch {
      setError("Mapping must be a JSON object.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await importDocument(selected.id, {
        mapping: parsedMapping,
        mode,
        backup_confirmed: true,
      });
      setReport(result);
      await queryClient.invalidateQueries({ queryKey: ["documents"] });
    } catch (importError) {
      setError(errorMessage(importError));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card" aria-label="documents">
      <h2>Documents</h2>
      <p className="muted">
        Statements and exports are stored locally, encrypted at rest, and tagged
        by institution/type. The original file is immutable; computed financial
        records are never deleted here.
      </p>

      <div className="bulk-actions">
        <label className="field">
          <span>Financial file</span>
          <input
            aria-label="financial file"
            type="file"
            accept=".csv,.xlsx,.xml,.ofx,.qfx,.pdf,application/pdf"
            disabled={busy}
            onChange={(event) => setFile(event.target.files?.[0] ?? null)}
          />
        </label>
        <label className="field">
          <span>Tags</span>
          <input
            aria-label="document tags"
            value={tags}
            onChange={(event) => setTags(event.target.value)}
          />
        </label>
        <button type="button" disabled={!file || busy} onClick={() => void handleUpload()}>
          {busy ? "Uploading …" : "Upload encrypted"}
        </button>
        <button
          type="button"
          disabled={documents.isFetching}
          onClick={() => void documents.refetch()}
        >
          Refresh
        </button>
      </div>

      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {notice && <p className="muted">{notice}</p>}
      {documents.isError && (
        <p className="error" role="alert">
          {errorMessage(documents.error)}
        </p>
      )}

      <table className="data-table" aria-label="uploaded documents">
        <thead>
          <tr>
            <th>Name</th>
            <th>Classification</th>
            <th>Status</th>
            <th>Tags</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {(documents.data ?? []).map((document) => (
            <tr key={document.id}>
              <td>{document.original_name}</td>
              <td>
                {document.institution ?? "Unclassified"}
                {document.document_type ? ` / ${document.document_type}` : ""}
                {document.classification_confidence !== null &&
                  ` (${Math.round(document.classification_confidence * 100)}%)`}
              </td>
              <td>{document.status}</td>
              <td>{document.tags.join(", ")}</td>
              <td>
                <button type="button" onClick={() => void openDocument(document)}>
                  Preview
                </button>
                <button type="button" onClick={() => void handleDownload(document)}>
                  Download
                </button>
                <button type="button" onClick={() => void handleTags(document)}>
                  Tags
                </button>
                <button type="button" onClick={() => void handleDelete(document)}>
                  Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {selected && (
        <div className="document-preview">
          <h3>{selected.original_name}</h3>
          {preview && (
            <>
              <p className="muted">
                Detected {preview.format} · {preview.institution ?? "unclassified"} ·{" "}
                {preview.document_type ?? "other"} ·{" "}
                {preview.confidence === null
                  ? "unknown confidence"
                  : `${Math.round(preview.confidence * 100)}%`}
              </p>
              {preview.error && <p className="error">{preview.error}</p>}
              {preview.table && (
                <div className="table-scroll">
                  <table className="data-table">
                    <thead>
                      <tr>
                        {preview.table.headers.map((header) => (
                          <th key={header}>{header}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {preview.table.rows.slice(0, 20).map((row, index) => (
                        <tr key={index}>
                          {row.map((cell, cellIndex) => (
                            <td key={cellIndex}>{cell}</td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </>
          )}

          <h4>Import preview</h4>
          <label className="field">
            <span>Field mapping JSON</span>
            <textarea
              aria-label="document mapping"
              value={mapping}
              onChange={(event) => setMapping(event.target.value)}
            />
          </label>
          <label className="field">
            <span>Mode</span>
            <select
              aria-label="document import mode"
              value={mode}
              onChange={(event) => setMode(event.target.value as "atomic" | "partial")}
            >
              <option value="atomic">Atomic (reject on any row error)</option>
              <option value="partial">Partial (write valid rows)</option>
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={backupConfirmed}
              onChange={(event) => setBackupConfirmed(event.target.checked)}
            />{" "}
            I took a verified backup with <code>make backup</code>
          </label>
          <div className="bulk-actions">
            <button
              type="button"
              disabled={busy || !backupConfirmed}
              onClick={() => void handleImport()}
            >
              Import document
            </button>
          </div>
          {report && (
            <p className="muted">
              Imported {report.accounts_created} account(s), {report.balances_saved}{" "}
              balance(s), {report.holding_lots_saved} lot(s),{" "}
              {report.crypto_lots_saved} crypto lot(s). Migration {report.migration_id}.
            </p>
          )}

          <h4>Audit</h4>
          <ul className="muted">
            {audit.map((event) => (
              <li key={event.id}>
                {event.created_at}: {event.action}
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
