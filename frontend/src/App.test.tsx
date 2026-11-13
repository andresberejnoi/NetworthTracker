import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

function renderApp(initialEntries: string[] = ["/"]) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={initialEntries}>
        <App />
      </MemoryRouter>
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
  as_of: "2026-11-12",
  currency: "USD",
  total: "100.00",
  liquid: "100.00",
  liquidity: "100.00",
  accessible: "100.00",
  by_account_type: [],
  by_asset_class: [],
  account_count: 0,
  has_manual_data: false,
  has_stale_data: false,
};

function stubApi(mode: "ok" | "error") {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (mode === "error") {
        throw new Error("connection refused");
      }
      if (url.includes("/api/v1/auth/status")) {
        return jsonResponse({
          auth_enabled: false,
          needs_setup: false,
          oidc_enabled: false,
          oidc_button_label: null,
        });
      }
      if (url.includes("/api/v1/auth/me")) {
        return jsonResponse({
          id: "11111111-1111-1111-1111-111111111111",
          username: "local",
          display_name: "Local User",
          role: "owner",
        });
      }
      if (url.includes("/api/v1/networth/snapshots")) {
        return jsonResponse([]);
      }
      if (url.includes("/api/v1/networth")) {
        return jsonResponse(networth);
      }
      if (url.includes("/api/v1/accounts")) {
        return jsonResponse([]);
      }
      return jsonResponse({});
    }),
  );
}

describe("App", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("renders the authenticated app shell and dashboard", async () => {
    stubApi("ok");
    renderApp();

    expect(await screen.findByText("NetworthTracker")).toBeInTheDocument();
    expect(screen.getByText("Your household's financial position at a glance.")).toBeInTheDocument();
    expect((await screen.findAllByText("100.00 USD")).length).toBeGreaterThan(0);
  });

  it("shows an error state when the backend is unreachable", async () => {
    stubApi("error");
    renderApp();

    expect(await screen.findByText(/connection refused/i)).toBeInTheDocument();
  });

  it("exchanges an OIDC callback code for a session", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.includes("/auth/status")) {
          return jsonResponse({
            auth_enabled: true,
            needs_setup: false,
            oidc_enabled: true,
            oidc_button_label: "Sign in with SSO",
          });
        }
        if (url.includes("/auth/oidc/exchange")) {
          return jsonResponse({
            token: "oidc-session",
            user: {
              id: "11111111-1111-1111-1111-111111111111",
              username: "sso-owner",
              display_name: "SSO Owner",
              role: "owner",
            },
          });
        }
        if (url.includes("/api/v1/networth/snapshots")) {
          return jsonResponse([]);
        }
        if (url.includes("/api/v1/networth")) {
          return jsonResponse(networth);
        }
        if (url.includes("/api/v1/accounts")) {
          return jsonResponse([]);
        }
        return jsonResponse({});
      }),
    );

    renderApp(["/auth/oidc/callback?code=one-time"]);
    expect(await screen.findByText("SSO Owner · owner")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByText("Your household's financial position at a glance.")).toBeInTheDocument(),
    );
  });
});
