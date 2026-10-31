import type { GridRow } from "../bulk-entry/BulkGrid";

/** Normalize a bulk row to the wallet identity it belongs to. */
export function walletGroupKey(row: GridRow): string {
  const kind = row.kind === "address" ? "address" : "manual";
  const identity =
    kind === "address"
      ? (row.address ?? "").trim().toLowerCase()
      : (row.label ?? "").trim().toLowerCase();
  return `${(row.asset ?? "").trim().toUpperCase()}|${kind}|${identity}`;
}

/** Group rows so repeated transactions/totals for one wallet share a wallet. */
export function groupWalletRows(
  rows: GridRow[],
): Array<{ key: string; entries: Array<{ row: GridRow; index: number }> }> {
  const groups = new Map<
    string,
    Array<{ row: GridRow; index: number }>
  >();
  rows.forEach((row, index) => {
    const key = walletGroupKey(row);
    const entries = groups.get(key) ?? [];
    entries.push({ row, index });
    groups.set(key, entries);
  });
  return [...groups.entries()].map(([key, entries]) => ({ key, entries }));
}
