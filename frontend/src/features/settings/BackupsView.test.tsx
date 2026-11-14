import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import BackupsView from "./BackupsView";

const summary = {
  timestamp: "20261113T120000Z",
  database: {
    name: "networth-20261113T120000Z.sql.gz.gpg",
    size_bytes: 2048,
    modified_at: "2026-11-13T12:00:00Z",
  },
  documents: {
    name: "networth-documents-20261113T120000Z.tar.gz.gpg",
    size_bytes: 512,
    modified_at: "2026-11-13T12:00:00Z",
  },
};

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function renderView() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <BackupsView />
    </QueryClientProvider>,
  );
}

describe("BackupsView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("lists backup archives and downloads them", async () => {
    const click = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(() => {});
    vi.stubGlobal("URL", {
      createObjectURL: vi.fn(() => "blob:backup"),
      revokeObjectURL: vi.fn(),
    });
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/download")) {
        return new Response(new Blob(["encrypted"]), { status: 200 });
      }
      return jsonResponse([summary]);
    });
    vi.stubGlobal("fetch", fetchMock);

    renderView();

    expect(await screen.findByText("2026-11-13 12:00:00 UTC")).toBeInTheDocument();
    expect(screen.getByText("2.0 KiB + 512 B")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Database" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining(`/api/v1/admin/backups/${summary.database.name}/download`),
        expect.any(Object),
      ),
    );
    expect(click).toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Documents" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining(
          `/api/v1/admin/backups/${summary.documents.name}/download`,
        ),
        expect.any(Object),
      ),
    );
  });

  it("creates a backup and reports success", async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === "POST") {
        return jsonResponse(summary, 201);
      }
      return jsonResponse([]);
    });
    vi.stubGlobal("fetch", fetchMock);

    renderView();

    await userEvent.click(await screen.findByRole("button", { name: "Create first backup" }));
    await waitFor(() =>
      expect(screen.getByText(/Backup created: 2026-11-13 12:00:00 UTC/)).toBeInTheDocument(),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/admin/backups"),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("shows a configuration error when creation fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
        if (init?.method === "POST") {
          return jsonResponse(
            { error: "BACKUP_PASSPHRASE is required to create an encrypted backup" },
            400,
          );
        }
        return jsonResponse([]);
      }),
    );

    renderView();

    await userEvent.click(await screen.findByRole("button", { name: "Create first backup" }));
    expect(
      await screen.findByText("BACKUP_PASSPHRASE is required to create an encrypted backup"),
    ).toBeInTheDocument();
  });

  it("shows a load error", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => jsonResponse({ error: "could not list backups" }, 500)),
    );

    renderView();

    expect(await screen.findByText("could not list backups")).toBeInTheDocument();
  });
});
