import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import AuthScreen from "./AuthScreen";

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

describe("AuthScreen", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("signs in and reports the authenticated user", async () => {
    const fetchMock = vi.fn(async () =>
      jsonResponse({
        token: "token-123",
        user: {
          id: "11111111-1111-1111-1111-111111111111",
          username: "owner",
          display_name: "Owner",
          role: "owner",
        },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);
    const onAuthenticated = vi.fn();

    render(<AuthScreen mode="login" onAuthenticated={onAuthenticated} />);
    await userEvent.type(screen.getByLabelText("username"), "owner");
    await userEvent.type(screen.getByLabelText("password"), "password123");
    await userEvent.click(screen.getByRole("button", { name: /sign in/i }));

    await waitFor(() =>
      expect(onAuthenticated).toHaveBeenCalledWith(
        expect.objectContaining({ username: "owner", role: "owner" }),
      ),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/auth/login"),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("shows an error for bad credentials", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        jsonResponse({ error: "invalid username or password" }, 401),
      ),
    );

    render(<AuthScreen mode="login" onAuthenticated={vi.fn()} />);
    await userEvent.type(screen.getByLabelText("username"), "owner");
    await userEvent.type(screen.getByLabelText("password"), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /sign in/i }));

    expect(
      await screen.findByText(/invalid username or password/i),
    ).toBeInTheDocument();
  });

  it("offers first-run setup", () => {
    render(<AuthScreen mode="setup" onAuthenticated={vi.fn()} />);

    expect(screen.getByText(/create the household owner/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /create owner/i }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("display name")).toBeInTheDocument();
  });
});
