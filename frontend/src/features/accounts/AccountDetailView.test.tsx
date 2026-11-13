import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import AccountDetailView from "./AccountDetailView";

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function renderView() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={["/accounts/11111111-1111-1111-1111-111111111111"]}>
        <Routes>
          <Route path="/accounts/:id" element={<AccountDetailView />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AccountDetailView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows account metadata and balance history", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.includes("/balances")) {
          return jsonResponse([
            {
              id: "22222222-2222-2222-2222-222222222222",
              account_id: "11111111-1111-1111-1111-111111111111",
              as_of: "2026-01-01",
              amount: "1234.56",
              currency: "USD",
              source: "manual",
            },
          ]);
        }
        return jsonResponse({
          id: "11111111-1111-1111-1111-111111111111",
          name: "Everyday Checking",
          account_type: "checking",
          institution: "Test Bank",
          currency: "USD",
          ownership_scope: "household",
          owner_user_id: null,
          archived: false,
        });
      }),
    );

    renderView();
    expect(await screen.findByText("Everyday Checking")).toBeInTheDocument();
    expect(await screen.findByText("1234.56 USD")).toBeInTheDocument();
    expect(screen.getByText("manual")).toBeInTheDocument();
  });
});
