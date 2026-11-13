import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { AuthStatus, User } from "../api/client";
import AppShell from "./AppShell";

const owner: User = {
  id: "11111111-1111-1111-1111-111111111111",
  username: "owner",
  display_name: "Owner",
  role: "owner",
};

const viewer: User = { ...owner, username: "viewer", display_name: "Viewer", role: "viewer" };

const status: AuthStatus = {
  auth_enabled: true,
  needs_setup: false,
  oidc_enabled: false,
  oidc_button_label: null,
};

function renderShell(user: User = owner) {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route
          element={<AppShell user={user} status={status} onLogout={vi.fn()} />}
        >
          <Route index element={<p>Home content</p>} />
          <Route path="accounts" element={<p>Accounts content</p>} />
          <Route path="settings/users" element={<p>Users content</p>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

describe("AppShell", () => {
  it("shows primary navigation and marks Home active", () => {
    renderShell();
    expect(
      screen.getAllByRole("link", { name: "Home" }).some((link) =>
        link.hasAttribute("aria-current"),
      ),
    ).toBe(true);
    expect(screen.getAllByRole("link", { name: "Accounts" }).length).toBeGreaterThan(0);
    expect(screen.getByText("Home content")).toBeInTheDocument();
  });

  it("opens and closes the mobile More sheet", async () => {
    renderShell();
    await userEvent.click(screen.getByRole("button", { name: "More" }));
    expect(screen.getByRole("dialog", { name: "More destinations" })).toBeInTheDocument();
    expect(screen.getAllByRole("link", { name: "Budget" }).length).toBeGreaterThan(0);
    await userEvent.keyboard("{Escape}");
    expect(
      screen.queryByRole("dialog", { name: "More destinations" }),
    ).not.toBeInTheDocument();
  });

  it("hides owner-only destinations from viewers", () => {
    renderShell(viewer);
    expect(screen.queryByRole("link", { name: "Settings" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Aggregation" })).not.toBeInTheDocument();
    expect(screen.getAllByRole("link", { name: "Budget" }).length).toBeGreaterThan(0);
  });
});
