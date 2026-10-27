import { useEffect, useRef, useState } from "react";
import type { ClipboardEvent, KeyboardEvent } from "react";

export interface GridColumn {
  key: string;
  label: string;
  placeholder?: string;
  validate?: (value: string) => string | null;
}

export type GridRow = Record<string, string>;

export interface SaveResult {
  savedCount: number;
  errors: { index: number; error: string }[];
}

interface BulkGridProps {
  columns: GridColumn[];
  minRows?: number;
  saveLabel?: string;
  readOnly?: boolean;
  onSave: (rows: GridRow[]) => Promise<SaveResult>;
}

function emptyRow(columns: GridColumn[]): GridRow {
  return Object.fromEntries(columns.map((column) => [column.key, ""]));
}

/**
 * A spreadsheet-like grid: one row per record, keyboard-first entry, and
 * clipboard paste of tab-separated blocks. Validation is inline and per row;
 * saving is a single action with a per-row result.
 */
export default function BulkGrid({
  columns,
  minRows = 1,
  saveLabel = "Save all",
  readOnly = false,
  onSave,
}: BulkGridProps) {
  const [rows, setRows] = useState<GridRow[]>(() =>
    Array.from({ length: minRows }, () => emptyRow(columns)),
  );
  const [cellErrors, setCellErrors] = useState<Record<string, string>>({});
  const [rowErrors, setRowErrors] = useState<Record<number, string>>({});
  const [focusTarget, setFocusTarget] = useState<{ row: number; col: number } | null>(
    null,
  );
  const [summary, setSummary] = useState<SaveResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const refs = useRef<Record<string, HTMLInputElement | null>>({});

  useEffect(() => {
    if (!focusTarget) return;
    const column = columns[focusTarget.col];
    if (!column) return;
    refs.current[`${focusTarget.row}:${column.key}`]?.focus();
    setFocusTarget(null);
  }, [focusTarget, columns, rows]);

  const updateCell = (rowIndex: number, key: string, value: string) => {
    setRows((current) =>
      current.map((row, index) =>
        index === rowIndex ? { ...row, [key]: value } : row,
      ),
    );
    setCellErrors((current) => {
      const next = { ...current };
      delete next[`${rowIndex}:${key}`];
      return next;
    });
    setRowErrors((current) => {
      if (!(rowIndex in current)) return current;
      const next = { ...current };
      delete next[rowIndex];
      return next;
    });
  };

  const addRow = () => {
    setRows((current) => [...current, emptyRow(columns)]);
    setFocusTarget({ row: rows.length, col: 0 });
  };

  const removeRow = (rowIndex: number) => {
    setRows((current) =>
      current.length <= 1
        ? current
        : current.filter((_, index) => index !== rowIndex),
    );
    setCellErrors({});
    setRowErrors({});
  };

  const validate = () => {
    const nextCellErrors: Record<string, string> = {};
    const nextRowErrors: Record<number, string> = {};

    rows.forEach((row, rowIndex) => {
      columns.forEach((column) => {
        const error = column.validate?.(row[column.key] ?? "");
        if (error) {
          nextCellErrors[`${rowIndex}:${column.key}`] = error;
        }
      });
      const first = columns
        .map((column) => nextCellErrors[`${rowIndex}:${column.key}`])
        .find(Boolean);
      if (first) {
        nextRowErrors[rowIndex] = first;
      }
    });

    setCellErrors(nextCellErrors);
    setRowErrors(nextRowErrors);
    return Object.keys(nextCellErrors).length === 0;
  };

  const save = async () => {
    setSummary(null);
    setMessage(null);

    if (!validate()) {
      setMessage("Fix the highlighted cells before saving.");
      return;
    }

    setIsSaving(true);
    try {
      const result = await onSave(rows);
      setSummary(result);
      if (result.errors.length > 0) {
        setRowErrors(
          Object.fromEntries(result.errors.map((error) => [error.index, error.error])),
        );
      }
    } catch (error) {
      setMessage((error as Error).message);
    } finally {
      setIsSaving(false);
    }
  };

  const handlePaste = (event: ClipboardEvent<HTMLDivElement>) => {
    const text = event.clipboardData.getData("text");
    if (!text.includes("\t") && !text.includes("\n")) return;

    event.preventDefault();
    const target = event.target as HTMLElement;
    const startRow = Number(target.dataset.row ?? 0);
    const startCol = Number(target.dataset.col ?? 0);
    const lines = text
      .replace(/\r/g, "")
      .split("\n")
      .filter((line) => line.length > 0);

    setRows((current) => {
      const next = [...current];
      while (next.length < startRow + lines.length) {
        next.push(emptyRow(columns));
      }
      lines.forEach((line, rowOffset) => {
        const cells = line.split("\t");
        const rowIndex = startRow + rowOffset;
        const merged = { ...next[rowIndex] };
        cells.forEach((cell, cellOffset) => {
          const column = columns[startCol + cellOffset];
          if (column) {
            merged[column.key] = cell.trim();
          }
        });
        next[rowIndex] = merged;
      });
      return next;
    });

    setSummary(null);
    setMessage(null);
  };

  const handleKeyDown = (
    event: KeyboardEvent<HTMLInputElement>,
    rowIndex: number,
    colIndex: number,
  ) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    if (rowIndex === rows.length - 1) {
      setRows((current) => [...current, emptyRow(columns)]);
    }
    setFocusTarget({ row: rowIndex + 1, col: colIndex });
  };

  return (
    <div className="bulk" onPaste={handlePaste}>
      <table className="table bulk-table">
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column.key}>{column.label}</th>
            ))}
            <th aria-label="row actions" />
          </tr>
        </thead>
        <tbody>
          {rows.map((row, rowIndex) => (
            <tr key={rowIndex} className={rowErrors[rowIndex] ? "row-invalid" : undefined}>
              {columns.map((column, colIndex) => {
                const key = `${rowIndex}:${column.key}`;
                const error = cellErrors[key];
                return (
                  <td key={column.key}>
                    <input
                      aria-label={`${column.label} row ${rowIndex + 1}`}
                      data-row={rowIndex}
                      data-col={colIndex}
                      disabled={readOnly}
                      ref={(element) => {
                        refs.current[key] = element;
                      }}
                      value={row[column.key] ?? ""}
                      placeholder={column.placeholder}
                      onChange={(event) =>
                        updateCell(rowIndex, column.key, event.target.value)
                      }
                      onKeyDown={(event) =>
                        handleKeyDown(event, rowIndex, colIndex)
                      }
                      aria-invalid={Boolean(error)}
                      className={error ? "invalid" : undefined}
                    />
                    {error && <span className="cell-error">{error}</span>}
                    {!error && rowErrors[rowIndex] && colIndex === 0 && (
                      <span className="cell-error">{rowErrors[rowIndex]}</span>
                    )}
                  </td>
                );
              })}
              <td>
                <button
                  type="button"
                  aria-label={`remove row ${rowIndex + 1}`}
                  onClick={() => removeRow(rowIndex)}
                  disabled={readOnly}
                >
                  ×
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="bulk-actions">
        <button type="button" onClick={addRow} disabled={readOnly}>
          Add row
        </button>
        <button type="button" onClick={save} disabled={isSaving || readOnly}>
          {isSaving ? "Saving…" : saveLabel}
        </button>
      </div>

      {message && (
        <p className="error" role="alert">
          {message}
        </p>
      )}

      {summary && (
        <div className="bulk-result" role="status">
          <p>
            Saved {summary.savedCount} row
            {summary.savedCount === 1 ? "" : "s"}.
          </p>
          {summary.errors.length > 0 && (
            <ul>
              {summary.errors.map((error) => (
                <li key={error.index}>
                  Row {error.index + 1}: {error.error}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
