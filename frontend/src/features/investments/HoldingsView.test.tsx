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
  lots: [],
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
  });

  it("creates a security", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByRole("button", { name: /delete VTI/i });
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
    expect(screen.getByRole("button", { name: "Add security" })).toBeDisabled();
  });
});
