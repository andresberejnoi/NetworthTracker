import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AuthProvider } from "../../auth/AuthContext";
import HoldingsView from "./HoldingsView";

const account = {
  id: "11111111-1111-1111-1111-111111111111",
  name: "Brokerage",
  account_type: "taxable_brokerage",
  institution: "Vanguard",
  currency: "USD",
  ownership_scope: "household",
  owner_user_id: null,
  archived: false,
};

const holding = {
  id: "22222222-2222-2222-2222-222222222222",
  account_id: account.id,
  symbol: "VTI",
  name: "Total Market",
  asset_class: "equity",
  sector: "other",
  currency: "USD",
  quantity: "10",
  cost_basis: "1000.00",
  price: "150.00",
  price_as_of: "2026-10-27T00:00:00Z",
  price_stale: false,
  price_source: "manual",
  market_value: "1500.00",
  unrealized_gain: "500.00",
  lots: [
    {
      id: "55555555-5555-5555-5555-555555555555",
      acquired_on: "2026-01-01",
      quantity: "10",
      original_quantity: "10",
      cost_basis: "1000.00",
      original_cost_basis: "1000.00",
      fees: "0.00",
      source: "purchase",
      basis_quality: "actual",
      basis_note: null,
    },
    {
      id: "66666666-6666-6666-6666-666666666666",
      acquired_on: "2026-02-01",
      quantity: "5",
      original_quantity: "5",
      cost_basis: "600.00",
      original_cost_basis: "600.00",
      fees: "0.00",
      source: "import",
      basis_quality: "estimated",
      basis_note: "backfilled",
    },
  ],
};

const security = {
  id: "33333333-3333-3333-3333-333333333333",
  symbol: "VTI",
  name: "Total Market",
  asset_class: "equity",
  sector: "other",
  currency: "USD",
};

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function stubApi() {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url.includes("/api/v1/accounts/") && url.includes("/holdings")) {
      return jsonResponse([holding]);
    }
    if (url.includes("/sales")) {
      if (init?.method === "POST") {
        return jsonResponse({
          id: "77777777-7777-7777-7777-777777777777",
          account_id: account.id,
          holding_id: holding.id,
          symbol: "VTI",
          name: "Total Market",
          disposed_on: "2026-06-01",
          quantity: "12",
          gross_proceeds: "1800.00",
          fees: "12.00",
          net_proceeds: "1788.00",
          cost_basis: "1240.00",
          realized_gain: "548.00",
          currency: "USD",
          method: "fifo",
          disposals: [
            {
              id: "88888888-8888-8888-8888-888888888888",
              lot_id: holding.lots[0].id,
              acquired_on: "2026-01-01",
              quantity: "10",
              gross_proceeds: "1500.00",
              fees: "10.00",
              net_proceeds: "1490.00",
              cost_basis: "1000.00",
              realized_gain: "490.00",
              holding_period_days: 151,
              term: "short",
              basis_quality: "actual",
            },
          ],
        });
      }
      return jsonResponse([]);
    }
    if (url.includes("/api/v1/accounts")) {
      return jsonResponse([account]);
    }
    if (url.includes("/api/v1/securities")) {
      return jsonResponse(init?.method === "POST" ? security : [security]);
    }
    if (url.includes("/api/v1/prices/provider")) {
      return jsonResponse({ configured: false, name: null });
    }
    return jsonResponse({});
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function renderView(role: "owner" | "viewer" = "owner") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <AuthProvider
      user={{
        id: "44444444-4444-4444-4444-444444444444",
        username: "user",
        display_name: "User",
        role,
      }}
    >
      <QueryClientProvider client={client}>
        <HoldingsView />
      </QueryClientProvider>
    </AuthProvider>,
  );
}

describe("HoldingsView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("lists holdings with valuation and allocation", async () => {
    stubApi();
    renderView();

    await screen.findByRole("button", { name: /delete VTI/i });
    expect(screen.getAllByText("1,500.00 USD").length).toBeGreaterThan(0);
    expect(screen.getByText("500.00 USD")).toBeInTheDocument();
    expect(screen.getByText("manual prices only")).toBeInTheDocument();
    expect(screen.getByText("Tax lots")).toBeInTheDocument();
    expect(screen.getAllByText("estimated").length).toBeGreaterThan(0);
    expect(screen.getByText("backfilled")).toBeInTheDocument();
  });

  it("creates a security", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByRole("button", { name: /delete VTI/i });
    await userEvent.click(screen.getByRole("button", { name: "New security" }));
    await userEvent.type(screen.getByLabelText("security symbol"), "VOO");
    await userEvent.type(screen.getByLabelText("security name"), "S&P 500");
    await userEvent.click(screen.getByRole("button", { name: "Add security" }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/securities"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  it("disables mutations for viewers", async () => {
    stubApi();
    renderView("viewer");

    const deleteButton = await screen.findByRole("button", {
      name: /delete VTI/i,
    });
    expect(deleteButton).toBeDisabled();
    expect(screen.getByRole("button", { name: "New security" })).toBeDisabled();
  });
  it("records a FIFO sale and shows the realized gain", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByRole("button", { name: /delete VTI/i });
    await userEvent.click(screen.getByRole("button", { name: "New sale" }));
    await userEvent.selectOptions(screen.getByLabelText("sale security"), "VTI");
    await userEvent.type(screen.getByLabelText("sale disposed date"), "2026-06-01");
    await userEvent.type(screen.getByLabelText("sale quantity"), "12");
    await userEvent.type(screen.getByLabelText("sale proceeds"), "1800.00");
    await userEvent.type(screen.getByLabelText("sale fees"), "12.00");
    await userEvent.click(screen.getByRole("button", { name: "Record sale" }));

    await waitFor(() =>
      expect(screen.getByText(/Realized gain: 548.00 USD/)).toBeInTheDocument(),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/accounts/"),
      expect.objectContaining({ method: "POST" }),
    );
  });

});
