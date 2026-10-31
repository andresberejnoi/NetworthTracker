import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AuthProvider } from "../../auth/AuthContext";
import AggregationView from "./AggregationView";

const empty = {
  status: {
    provider: null,
    enabled: false,
    has_token: false,
    provider_available: true,
  },
  links: [],
};

const synced = {
  status: {
    provider: "fake",
    enabled: true,
    has_token: true,
    provider_available: true,
  },
  links: [
    {
      id: "11111111-1111-1111-1111-111111111111",
      provider: "fake",
      provider_account_id: "fake-brokerage-1",
      account_id: "22222222-2222-2222-2222-222222222222",
      display_name: "Fake Brokerage",
      account_type: "taxable_brokerage",
      currency: "USD",
      last_synced_at: "2026-10-31T00:00:00Z",
      provider_balance: "12345.67",
      provider_as_of: "2026-10-31",
      local_balance: "999.00",
      local_source: "manual",
      in_effect: "manual",
      holdings: [
        {
          symbol: "VTI",
          quantity: "10",
          value: "1500.00",
          currency: "USD",
          synced_at: "2026-10-31T00:00:00Z",
        },
      ],
    },
  ],
};

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function stubApi() {
  let configured = false;
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.includes("/api/v1/aggregation/sync")) {
      return jsonResponse(synced);
    }
    if (url.includes("/api/v1/aggregation/config")) {
      configured = true;
      return jsonResponse(synced.status);
    }
    if (url.includes("/api/v1/aggregation")) {
      return jsonResponse(configured ? synced : empty);
    }
    return jsonResponse({});
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function renderView() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <AuthProvider
      user={{
        id: "44444444-4444-4444-4444-444444444444",
        username: "owner",
        display_name: "Owner",
        role: "owner",
      }}
    >
      <QueryClientProvider client={client}>
        <AggregationView />
      </QueryClientProvider>
    </AuthProvider>,
  );
}

describe("AggregationView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows the unconfigured state", async () => {
    stubApi();
    renderView();

    await screen.findByText("No linked accounts yet.");
    expect(screen.getByText("disabled")).toBeInTheDocument();
    expect(screen.getByText("provider adapter available")).toBeInTheDocument();
  });

  it("saves configuration and syncs", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByText("No linked accounts yet.");
    await userEvent.type(screen.getByLabelText("aggregation token"), "test-token");
    await userEvent.click(screen.getByRole("button", { name: "Save config" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/aggregation/config"),
        expect.objectContaining({ method: "POST" }),
      ),
    );

    await userEvent.click(screen.getByRole("button", { name: "Sync now" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/aggregation/sync"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  it("renders provenance and provider holdings", async () => {
    stubApi();
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/v1/aggregation")) {
        return jsonResponse(synced);
      }
      return jsonResponse({});
    });
    renderView();

    expect((await screen.findAllByText(/Fake Brokerage/)).length).toBeGreaterThan(0);
    expect(screen.getByText("12,345.67 USD")).toBeInTheDocument();
    expect(screen.getByText("999.00 USD")).toBeInTheDocument();
    expect(screen.getAllByText("manual").length).toBeGreaterThan(0);
    expect(screen.getByText("VTI")).toBeInTheDocument();
    expect(screen.getByText("1,500.00 USD")).toBeInTheDocument();
  });
});
