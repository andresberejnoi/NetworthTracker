import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

function renderApp() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function stubApi(ping: "ok" | "error") {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/v1/auth/status")) {
        return jsonResponse({ auth_enabled: false, needs_setup: false });
      }
      if (url.includes("/api/v1/auth/me")) {
        return jsonResponse({
          id: "11111111-1111-1111-1111-111111111111",
          username: "local",
          display_name: "Local User",
          role: "owner",
        });
      }
      if (url.includes("/api/v1/ping")) {
        if (ping === "error") {
          throw new Error("connection refused");
        }
        return jsonResponse({
          message: "pong",
          service: "NetworthTracker",
          version: "0.1.20261110",
        });
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

  it("renders backend metadata returned by the API", async () => {
    stubApi("ok");
    renderApp();

    expect(await screen.findByText("NetworthTracker")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("pong")).toBeInTheDocument());
  });

  it("shows an error state when the backend is unreachable", async () => {
    stubApi("error");
    renderApp();

    expect(await screen.findByText(/unreachable/i)).toBeInTheDocument();
  });

  it("exchanges an OIDC callback code for a session", async () => {
    window.history.pushState({}, "", "/auth/oidc/callback?code=one-time");
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
        if (url.includes("/api/v1/ping")) {
          return jsonResponse({
            message: "pong",
            service: "NetworthTracker",
            version: "0.1.20261110",
          });
        }
        return jsonResponse({});
      }),
    );

    renderApp();
    expect(await screen.findByText("NetworthTracker")).toBeInTheDocument();
    expect(await screen.findByText(/pong/i)).toBeInTheDocument();
    window.history.replaceState({}, "", "/");
  });
});
