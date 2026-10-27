import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import UsersView from "./UsersView";

function renderView() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <UsersView />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const owner = {
  id: "11111111-1111-1111-1111-111111111111",
  username: "owner",
  display_name: "Owner",
  role: "owner",
};

describe("UsersView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("lists household members", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => jsonResponse([owner])));

    renderView();

    expect(await screen.findByText("Owner")).toBeInTheDocument();
    expect(screen.getAllByText("owner").length).toBeGreaterThan(0);
  });

  it("adds a member", async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === "POST") {
        return jsonResponse(
          { ...owner, id: "22222222-2222-2222-2222-222222222222", username: "kid", display_name: "Kid", role: "viewer" },
          201,
        );
      }
      return jsonResponse([owner]);
    });
    vi.stubGlobal("fetch", fetchMock);

    renderView();
    await screen.findByText("Owner");

    await userEvent.type(screen.getByLabelText("new username"), "kid");
    await userEvent.type(screen.getByLabelText("new display name"), "Kid");
    await userEvent.type(screen.getByLabelText("new password"), "test-only-password");
    await userEvent.selectOptions(screen.getByLabelText("new role"), "viewer");
    await userEvent.click(screen.getByRole("button", { name: /add member/i }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/users"),
        expect.objectContaining({ method: "POST" }),
      ),
    );
  });
});
