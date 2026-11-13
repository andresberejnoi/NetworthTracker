import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { AuthProvider } from "../auth/AuthContext";
import type { User } from "../api/client";
import DataLayout from "./DataLayout";

const owner: User = {
  id: "11111111-1111-1111-1111-111111111111",
  username: "owner",
  display_name: "Owner",
  role: "owner",
};
const viewer: User = { ...owner, username: "viewer", role: "viewer" };

function renderLayout(user: User) {
  return render(
    <AuthProvider user={user}>
      <MemoryRouter initialEntries={["/data/import-export"]}>
        <Routes>
          <Route path="/data" element={<DataLayout />}>
            <Route path="import-export" element={<p>import content</p>} />
            <Route path="migration" element={<p>migration content</p>} />
            <Route path="documents" element={<p>documents content</p>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </AuthProvider>,
  );
}

describe("DataLayout", () => {
  it("shows owner-only migration and document tabs to owners", () => {
    renderLayout(owner);
    expect(screen.getAllByRole("link", { name: "Import / export" }).length).toBeGreaterThan(0);
    expect(screen.getAllByRole("link", { name: "Migration" }).length).toBeGreaterThan(0);
    expect(screen.getAllByRole("link", { name: "Documents" }).length).toBeGreaterThan(0);
  });

  it("hides migration and documents from viewers", () => {
    renderLayout(viewer);
    expect(screen.queryByRole("link", { name: "Migration" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Documents" })).not.toBeInTheDocument();
  });
});
