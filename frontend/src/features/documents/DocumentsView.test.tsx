import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import DocumentsView from "./DocumentsView";

function renderView() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <DocumentsView />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const document = {
  id: "11111111-1111-1111-1111-111111111111",
  sha256: "a".repeat(64),
  original_name: "fidelity-holdings.csv",
  mime_type: "text/csv",
  byte_size: 100,
  uploader_user_id: null,
  tags: ["retirement"],
  institution: "Fidelity",
  document_type: "holdings",
  classification_confidence: 0.8,
  parser_version: "generic-1",
  status: "parsed",
  period: "2026",
  parse_error: null,
  nonce: "nonce",
  key_version: "1",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  deleted_at: null,
};

describe("DocumentsView", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("lists documents and previews the detected table", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/v1/documents?include_deleted=true")) {
        return jsonResponse([document]);
      }
      if (url.includes("/preview")) {
        return jsonResponse({
          document,
          format: "csv",
          institution: "Fidelity",
          document_type: "holdings",
          confidence: 0.8,
          table: {
            headers: ["record_type", "account_name"],
            rows: [["account", "Checking"]],
          },
          error: null,
          audit: [],
        });
      }
      if (url.includes("/audit")) {
        return jsonResponse([]);
      }
      return jsonResponse({});
    });
    vi.stubGlobal("fetch", fetchMock);

    renderView();
    expect(await screen.findByText("fidelity-holdings.csv")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Preview" }));

    expect(await screen.findByText(/Detected csv/)).toBeInTheDocument();
    expect(screen.getByText("account_name")).toBeInTheDocument();
    expect(screen.getByText("Checking")).toBeInTheDocument();
  });
});
