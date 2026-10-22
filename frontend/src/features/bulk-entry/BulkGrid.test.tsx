import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import BulkGrid, { type GridColumn, type SaveResult } from "./BulkGrid";

const columns: GridColumn[] = [
  {
    key: "as_of",
    label: "Date",
    placeholder: "YYYY-MM-DD",
    validate: (value) => (value === "" ? "date is required" : null),
  },
  {
    key: "amount",
    label: "Amount",
    placeholder: "0.00",
    validate: (value) => (value === "" ? "amount is required" : null),
  },
];

function renderGrid(onSave: (rows: Record<string, string>[]) => Promise<SaveResult>) {
  return render(<BulkGrid columns={columns} onSave={onSave} />);
}

async function fillFirstRow(date = "2026-01-01", amount = "10.00") {
  await userEvent.type(screen.getByLabelText("Date row 1"), date);
  await userEvent.type(screen.getByLabelText("Amount row 1"), amount);
}

describe("BulkGrid", () => {
  it("adds the next row when Enter is pressed on the last row", async () => {
    renderGrid(async () => ({ savedCount: 0, errors: [] }));

    await userEvent.type(screen.getByLabelText("Date row 1"), "2026-01-01{Enter}");

    await waitFor(() =>
      expect(screen.getByLabelText("Date row 2")).toHaveFocus(),
    );
  });

  it("pastes a block of twenty rows", () => {
    renderGrid(async () => ({ savedCount: 0, errors: [] }));

    const block = Array.from(
      { length: 20 },
      (_, index) => `2026-01-${String(index + 1).padStart(2, "0")}\t${index + 1}.00`,
    ).join("\n");
    const clipboardData = { getData: () => block } as unknown as DataTransfer;

    fireEvent.paste(screen.getByLabelText("Date row 1"), { clipboardData });

    expect(screen.getAllByLabelText(/^Date row /)).toHaveLength(20);
    expect(screen.getByLabelText("Date row 20")).toHaveValue("2026-01-20");
    expect(screen.getByLabelText("Amount row 20")).toHaveValue("20.00");
  });

  it("blocks saving while a cell is invalid", async () => {
    const onSave = vi.fn(async () => ({ savedCount: 0, errors: [] }));
    renderGrid(onSave);

    await userEvent.click(screen.getByRole("button", { name: /save all/i }));

    expect(onSave).not.toHaveBeenCalled();
    expect(
      screen.getByText(/fix the highlighted cells/i),
    ).toBeInTheDocument();
    expect(screen.getAllByText("date is required").length).toBeGreaterThan(0);
  });

  it("saves all valid rows in one action", async () => {
    const onSave = vi.fn(async () => ({ savedCount: 1, errors: [] }));
    renderGrid(onSave);

    await fillFirstRow();
    await userEvent.click(screen.getByRole("button", { name: /save all/i }));

    expect(onSave).toHaveBeenCalledWith([
      { as_of: "2026-01-01", amount: "10.00" },
    ]);
    expect(await screen.findByText("Saved 1 row.")).toBeInTheDocument();
  });

  it("reports rows the server skipped", async () => {
    const onSave = vi.fn(async () => ({
      savedCount: 0,
      errors: [{ index: 0, error: "currency mismatch: account uses USD, row uses EUR" }],
    }));
    renderGrid(onSave);

    await fillFirstRow();
    await userEvent.click(screen.getByRole("button", { name: /save all/i }));

    expect(
      await screen.findByText(/Row 1: currency mismatch/),
    ).toBeInTheDocument();
    expect(screen.getByText("Saved 0 rows.")).toBeInTheDocument();
  });
});
