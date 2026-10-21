import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import NetWorthView from "./NetWorthView";

function renderView() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <NetWorthView />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const networth = {
  as_of: "2026-10-17",
  currency: "USD",
  total: "610000.00",
  liquid: "150000.00",
  liquidity: "10000.00",
  accessible: "550000.00",
  by_account_type: [
    { key: "401k", amount: "100000.00", currency: "USD" },
    { key: "cash", amount: "10000.00", currency: "USD" },
  ],
  by_asset_class: [
    { key: "cash", amount: "10000.00", currency: "USD" },
    { key: "market", amount: "100000.00", currency: "USD" },
    { key: "real_estate", amount: "500000.00", currency: "USD" },
  ],
  account_count: 3,
  has_manual_data: true,
  has_stale_data: false,
};

const policy = {
  tax_rate: "0.2",
  penalty_rate: "0.1",
  penalty_age: 59,
  selling_rate: "0.06",
  fixed_costs: "0",
};

const snapshots = [
  {
    id: "11111111-1111-1111-1111-111111111111",
    as_of: "2026-09-01",
    scope: "household",
    owner_user_id: null,
    owner_age: 35,
    currency: "USD",
    total: "600000.00",
    liquid: "140000.00",
    liquidity: "9000.00",
    accessible: "540000.00",
    policy,
  },
  {
    id: "22222222-2222-2222-2222-222222222222",
    as_of: "2026-10-17",
    scope: "household",
    owner_user_id: null,
    owner_age: 35,
    currency: "USD",
    total: "610000.00",
    liquid: "150000.00",
    liquidity: "10000.00",
    accessible: "550000.00",
    policy,
  },
];

function stubApi() {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url.includes("/api/v1/networth/snapshots")) {
      if (init?.method === "POST") {
        return jsonResponse(snapshots[1], 201);
      }
      return jsonResponse(snapshots);
    }
    if (url.includes("/api/v1/networth")) {
      return jsonResponse(networth);
    }
    return jsonResponse({});
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("NetWorthView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows total, liquid, and accessible net worth", async () => {
    stubApi();
    renderView();

    expect(await screen.findByText("610,000.00 USD")).toBeInTheDocument();
    expect(screen.getByText("150,000.00 USD")).toBeInTheDocument();
    expect(screen.getAllByText("10,000.00 USD").length).toBeGreaterThan(0);
    expect(screen.getByText("550,000.00 USD")).toBeInTheDocument();
  });

  it("shows the breakdown and manual-data badge", async () => {
    stubApi();
    renderView();

    expect(await screen.findByText("real_estate")).toBeInTheDocument();
    expect(screen.getByText("manual entries")).toBeInTheDocument();
  });

  it("switches the breakdown grouping", async () => {
    stubApi();
    renderView();

    await screen.findByText("real_estate");
    await userEvent.selectOptions(screen.getByLabelText("group by"), "by_account_type");

    expect(await screen.findByText("401k")).toBeInTheDocument();
  });

  it("renders the history chart when snapshots exist", async () => {
    stubApi();
    renderView();

    expect(
      await screen.findByRole("img", { name: /net worth history/i }),
    ).toBeInTheDocument();
  });

  it("takes a snapshot", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByText("610,000.00 USD");
    await userEvent.click(screen.getByRole("button", { name: /take snapshot/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/networth/snapshots"),
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("keeps the previous figures visible while a new age loads", async () => {
    let release: (() => void) | undefined;

    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.includes("/api/v1/networth/snapshots")) {
          return jsonResponse(snapshots);
        }
        if (url.includes("owner_age=65")) {
          await new Promise<void>((resolve) => {
            release = resolve;
          });
          return jsonResponse({
            ...networth,
            total: "700000.00",
            liquid: "200000.00",
            liquidity: "20000.00",
            accessible: "600000.00",
          });
        }
        return jsonResponse(networth);
      }),
    );

    renderView();
    expect(await screen.findByText("610,000.00 USD")).toBeInTheDocument();

    await userEvent.clear(screen.getByLabelText("owner age"));
    await userEvent.type(screen.getByLabelText("owner age"), "65");

    // The refetch is still in flight, but the previous values stay on screen.
    expect(screen.getByText("610,000.00 USD")).toBeInTheDocument();
    expect(screen.queryByText("—")).not.toBeInTheDocument();

    await waitFor(() => expect(typeof release).toBe("function"));
    release?.();

    expect(await screen.findByText("700,000.00 USD")).toBeInTheDocument();
  });
});
