import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import AccountsView from "./AccountsView";

function renderView() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <AccountsView />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const checking = {
  id: "11111111-1111-1111-1111-111111111111",
  name: "Everyday Checking",
  account_type: "checking",
  institution: "Test Bank",
  currency: "USD",
  ownership_scope: "household",
  archived: false,
};

describe("AccountsView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("lists accounts returned by the API", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => jsonResponse([checking])));

    renderView();

    expect(await screen.findByText("Everyday Checking")).toBeInTheDocument();
    expect(screen.getByText("Test Bank")).toBeInTheDocument();
  });

  it("shows an empty state when there are no accounts", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => jsonResponse([])));

    renderView();

    expect(await screen.findByText(/no accounts yet/i)).toBeInTheDocument();
  });

  it("creates an account and refreshes the list", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (init?.method === "POST") {
        return jsonResponse(
          { ...checking, name: "New Brokerage", account_type: "taxable_brokerage" },
          201,
        );
      }
      void url;
      return jsonResponse([]);
    });
    vi.stubGlobal("fetch", fetchMock);

    renderView();

    await screen.findByText(/no accounts yet/i);
    await userEvent.type(
      screen.getByLabelText("account name"),
      "New Brokerage",
    );
    await userEvent.click(screen.getByRole("button", { name: /add account/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/accounts"),
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("surfaces API validation errors", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
        if (init?.method === "POST") {
          return jsonResponse({ error: "unknown account type: nope" }, 400);
        }
        return jsonResponse([]);
      }),
    );

    renderView();

    await screen.findByText(/no accounts yet/i);
    await userEvent.type(screen.getByLabelText("account name"), "Bad");
    await userEvent.click(screen.getByRole("button", { name: /add account/i }));

    expect(
      await screen.findByText(/unknown account type/i),
    ).toBeInTheDocument();
  });
});
