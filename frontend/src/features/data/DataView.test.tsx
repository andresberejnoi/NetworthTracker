import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import DataView from "./DataView";

function renderView() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <DataView />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

describe("DataView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("imports a CSV and shows the report", async () => {
    const fetchMock = vi.fn(async () =>
      jsonResponse({ accounts_created: 1, balances_saved: 2, errors: [] }),
    );
    vi.stubGlobal("fetch", fetchMock);

    renderView();
    const file = new File(["schema_version\n"], "sample.csv", {
      type: "text/csv",
    });
    await userEvent.upload(screen.getByLabelText("spreadsheet file"), file);

    expect(await screen.findByText(/Created 1 account/)).toBeInTheDocument();
    expect(screen.getByText(/saved 2 balances/)).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/import?format=csv"),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("shows row-level errors and that nothing was written", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        jsonResponse({
          accounts_created: 0,
          balances_saved: 0,
          errors: [{ row: 3, error: "invalid amount: oops" }],
        }),
      ),
    );

    renderView();
    const file = new File(["x"], "sample.xlsx", {
      type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    });
    await userEvent.upload(screen.getByLabelText("spreadsheet file"), file);

    expect(await screen.findByText("Line 3: invalid amount: oops")).toBeInTheDocument();
    expect(screen.getByText(/nothing was written/i)).toBeInTheDocument();
  });

  it("links to the export and template downloads", () => {
    renderView();

    expect(
      screen.getByRole("link", { name: "Export CSV" }).getAttribute("href"),
    ).toContain("/api/v1/export?format=csv");
    expect(
      screen.getByRole("link", { name: "Export XLSX" }).getAttribute("href"),
    ).toContain("/api/v1/export?format=xlsx");
    expect(
      screen.getByRole("link", { name: "Template CSV" }).getAttribute("href"),
    ).toContain("/api/v1/template?format=csv");
  });
});
