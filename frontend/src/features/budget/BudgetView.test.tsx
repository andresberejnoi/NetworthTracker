import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AuthProvider } from "../../auth/AuthContext";
import BudgetView from "./BudgetView";

const budget = {
  year: 2026,
  currency: "USD",
  income: "120000.00",
  allocations: [
    { category: "fixed_costs", percentage: "50", planned: "60000.00" },
    { category: "discretionary", percentage: "15", planned: "18000.00" },
  ],
  total_percentage: "65",
  balanced: false,
  over_allocated: false,
  spending: [],
  variance: [
    {
      category: "fixed_costs",
      planned: "60000.00",
      actual: "1000.00",
      variance_amount: "59000.00",
      variance_percent: "98.33333333333333333333333333",
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
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.includes("/api/v1/budget/income")) {
      return new Response(null, { status: 204 });
    }
    if (url.includes("/api/v1/budget/plan")) {
      return new Response(null, { status: 204 });
    }
    if (url.includes("/api/v1/budget/spending")) {
      return new Response(null, { status: 204 });
    }
    if (url.includes("/api/v1/budget")) {
      return jsonResponse(budget);
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
        <BudgetView />
      </QueryClientProvider>
    </AuthProvider>,
  );
}

describe("BudgetView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows income, plan, and variance", async () => {
    stubApi();
    renderView();

    await screen.findByPlaceholderText("120000.00");
    await waitFor(() =>
      expect(screen.getByPlaceholderText("120000.00")).toHaveValue("120000.00"),
    );
    expect(screen.getAllByText("60,000.00 USD").length).toBeGreaterThan(0);
    expect(screen.getByText("1,000.00 USD")).toBeInTheDocument();
    expect(screen.getByText("59,000.00 USD")).toBeInTheDocument();
    expect(screen.getByText("allocated 65%")).toBeInTheDocument();
  });

  it("saves income", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByPlaceholderText("120000.00");
    const input = screen.getByPlaceholderText("120000.00");
    await userEvent.clear(input);
    await userEvent.type(input, "150000.00");
    await userEvent.click(screen.getByRole("button", { name: "Save income" }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/budget/income"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  it("saves a plan and spending", async () => {
    const fetchMock = stubApi();
    renderView();

    await screen.findByPlaceholderText("120000.00");
    await userEvent.type(screen.getByLabelText("Savings percentage"), "10");
    await userEvent.click(screen.getByRole("button", { name: "Save plan" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/budget/plan"),
        expect.objectContaining({ method: "POST" }),
      ),
    );

    await userEvent.type(screen.getByLabelText("budget spending amount"), "250.00");
    await userEvent.click(screen.getByRole("button", { name: "Record spending" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/budget/spending"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });

  it("disables writes for viewers", async () => {
    stubApi();
    renderView("viewer");

    await screen.findByPlaceholderText("120000.00");
    expect(screen.getByRole("button", { name: "Save income" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save plan" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Record spending" })).toBeDisabled();
  });
});
