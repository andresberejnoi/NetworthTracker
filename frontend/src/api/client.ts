import type { components } from "./schema";

/**
 * Base URL of the backend API.
 *
 * The frontend is a pure consumer of the backend: it only ever talks to the
 * API over HTTP and never imports backend code. Configure the URL with
 * `VITE_API_BASE_URL` (see `.env.example`).
 */
export const API_BASE_URL =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) ??
  "http://localhost:8080";

/** Typed against the shared OpenAPI contract so drift is caught by `check:api`. */
export type PingResponse = components["schemas"]["Ping"];

export async function getPing(signal?: AbortSignal): Promise<PingResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/ping`, { signal });
  if (!response.ok) {
    throw new Error(`backend returned ${response.status}`);
  }
  return (await response.json()) as PingResponse;
}
