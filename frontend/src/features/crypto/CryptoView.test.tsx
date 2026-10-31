import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AuthProvider } from "../../auth/AuthContext";
import CryptoView from "./CryptoView";

const asset = {
  id: "11111111-1111-1111-1111-111111111111",
  symbol: "BTC",
  name: "Bitcoin",
  chain: "bitcoin",
  contract_address: null,
  decimals: 18,
  currency: "USD",
  price_provider_id: "btc-bitcoin",
};

const wallet = {
  id: "22222222-2222-2222-2222-222222222222",
  asset_id: asset.id,
  symbol: "BTC",
  name: "Bitcoin",
  chain: "bitcoin",
  kind: "manual",
  address: null,
  label: "Coinbase",
  ownership_scope: "household",
  owner_user_id: null,
  archived: false,
};

const position = {
  asset_id: asset.id,
  symbol: "BTC",
  name: "Bitcoin",
  chain: "bitcoin",
  quantity: "0.003",
  wallet_count: 1,
  price: "70000.00",
  price_as_of: "2026-10-30T00:00:00Z",
  price_stale: false,
  price_source: "manual",
  value: "210.00",
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
    if (url.includes("/api/v1/crypto/assets")) {
      return jsonResponse(init?.method === "POST" ? asset : [asset]);
    }
    if (url.includes("/balances")) {
      return jsonResponse([
        {
          id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
          wallet_id: wallet.id,
          as_of: "2026-01-01",
          quantity: "1.5",
          source: "manual",
          provider: null,
          fetched_at: null,
        },
      ]);
    }
    if (url.includes("/lots")) {
      return jsonResponse([
        {
          id: "99999999-9999-9999-9999-999999999999",
          acquired_on: "2026-01-01",
          quantity: "0.003",
          original_quantity: "0.003",
          cost_basis: "210.00",
          original_cost_basis: "210.00",
          fees: "0.00",
          source: "purchase",
          basis_quality: "actual",
          basis_note: null,
        },
      ]);
    }
    if (url.includes("/sales")) {
      return jsonResponse([]);
    }
    if (url.includes("/api/v1/crypto/wallets")) {
      return jsonResponse(init?.method === "POST" ? wallet : [wallet]);
    }
    if (url.includes("/api/v1/crypto/positions")) {
      return jsonResponse([position]);
    }
    if (url.includes("/api/v1/crypto/provider")) {
      return jsonResponse({ configured: false, name: null });
    }
    if (url.includes("/api/v1/prices")) {
      return jsonResponse({});
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
        <CryptoView />
      </QueryClientProvider>
    </AuthProvider>,
  );
}

describe("CryptoView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows positions, wallets, and the offline provider badge", async () => {
    stubApi();
    renderView();

    await screen.findByText("Coinbase");
    expect(screen.getAllByText("0.003").length).toBeGreaterThan(0);
    expect(screen.getAllByText("210.00 USD").length).toBeGreaterThan(0);
    expect(screen.getByText("manual balances only")).toBeInTheDocument();
    expect(screen.getAllByText("BTC").length).toBeGreaterThan(0);
  });

  it("creates an asset", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByText("Coinbase");
    await userEvent.type(screen.getByLabelText("crypto asset symbol"), "ETH");
    await userEvent.type(screen.getByLabelText("crypto asset name"), "Ethereum");
    await userEvent.selectOptions(screen.getByLabelText("crypto asset chain"), "ethereum");
    await userEvent.click(screen.getByRole("button", { name: "Add asset" }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/crypto/assets"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  it("records a manual balance", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByText("Coinbase");
    await userEvent.selectOptions(
      screen.getByLabelText("crypto balance wallet"),
      wallet.id,
    );
    await userEvent.type(screen.getByLabelText("crypto balance date"), "2026-01-01");
    await userEvent.type(screen.getByLabelText("crypto balance quantity"), "0.5");
    await userEvent.click(screen.getByRole("button", { name: "Record balance" }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/balances"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  it("disables mutations for viewers", async () => {
    stubApi();
    renderView("viewer");

    await screen.findByText("Coinbase");
    expect(screen.getByRole("button", { name: "Add asset" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add wallet" })).toBeDisabled();
  });
  it("shows crypto tax lots for a selected wallet", async () => {
    stubApi();
    renderView();

    await screen.findByText("Coinbase");
    await userEvent.selectOptions(
      screen.getByLabelText("crypto tax wallet"),
      wallet.id,
    );
    await screen.findByText("Crypto tax lots");
    expect(screen.getAllByText("0.003").length).toBeGreaterThan(0);
    expect(screen.getAllByText("210.00 USD").length).toBeGreaterThan(0);
  });

  it("warns when tax lots differ from the wallet balance and can prefill an opening lot", async () => {
    stubApi();
    renderView();

    await screen.findByText("Coinbase");
    await userEvent.selectOptions(
      screen.getByLabelText("crypto tax wallet"),
      wallet.id,
    );
    await screen.findByText(/differs from wallet balance/);
    await userEvent.click(
      screen.getByRole("button", { name: "Use balance as opening lot" }),
    );
    expect(screen.getByLabelText("crypto lot quantity")).toHaveValue("1.5");
  });

});
