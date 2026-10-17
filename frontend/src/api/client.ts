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

/** All types below are generated from `contracts/openapi.yaml`. */
export type PingResponse = components["schemas"]["Ping"];
export type Account = components["schemas"]["Account"];
export type AccountType = components["schemas"]["AccountType"];
export type OwnershipScope = components["schemas"]["OwnershipScope"];
export type Balance = components["schemas"]["Balance"];
export type BalanceSource = components["schemas"]["BalanceSource"];
export type CreateAccountRequest = components["schemas"]["CreateAccountRequest"];
export type RecordBalanceRequest = components["schemas"]["RecordBalanceRequest"];

/** Every account type the backend accepts, for form controls. */
export const ACCOUNT_TYPES: AccountType[] = [
  "401k",
  "traditional_ira",
  "roth_ira",
  "taxable_brokerage",
  "checking",
  "savings",
  "cash",
  "real_estate",
  "other_asset",
  "mortgage",
  "loan",
];

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: { "content-type": "application/json" },
    ...init,
  });

  if (!response.ok) {
    let message = `backend returned ${response.status}`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body?.error) {
        message = body.error;
      }
    } catch {
      // keep the status-based message
    }
    throw new Error(message);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

export function getPing(signal?: AbortSignal): Promise<PingResponse> {
  return request<PingResponse>("/api/v1/ping", { signal });
}

export function listAccounts(includeArchived = false): Promise<Account[]> {
  return request<Account[]>(
    `/api/v1/accounts?include_archived=${includeArchived}`,
  );
}

export function createAccount(body: CreateAccountRequest): Promise<Account> {
  return request<Account>("/api/v1/accounts", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function getAccount(id: string): Promise<Account> {
  return request<Account>(`/api/v1/accounts/${id}`);
}

export function archiveAccount(id: string): Promise<void> {
  return request<void>(`/api/v1/accounts/${id}/archive`, { method: "POST" });
}

export function listBalances(id: string): Promise<Balance[]> {
  return request<Balance[]>(`/api/v1/accounts/${id}/balances`);
}

export function recordBalance(
  id: string,
  body: RecordBalanceRequest,
): Promise<Balance> {
  return request<Balance>(`/api/v1/accounts/${id}/balances`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export type AggregationScope = components["schemas"]["AggregationScope"];
export type NetWorth = components["schemas"]["NetWorth"];
export type BreakdownRow = components["schemas"]["BreakdownRow"];
export type Snapshot = components["schemas"]["Snapshot"];
export type SnapshotRequest = components["schemas"]["SnapshotRequest"];

export interface NetWorthParams {
  as_of?: string;
  scope?: AggregationScope;
  owner_user_id?: string;
  owner_age?: number;
  tax_rate?: string;
  penalty_rate?: string;
  penalty_age?: number;
  selling_rate?: string;
  fixed_costs?: string;
}

function toQuery(params: object): string {
  const query = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      query.set(key, String(value));
    }
  }
  const encoded = query.toString();
  return encoded ? `?${encoded}` : "";
}

export function getNetWorth(params: NetWorthParams = {}): Promise<NetWorth> {
  return request<NetWorth>(`/api/v1/networth${toQuery(params)}`);
}

export function createSnapshot(body: SnapshotRequest): Promise<Snapshot> {
  return request<Snapshot>("/api/v1/networth/snapshots", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function listSnapshots(
  params: { scope?: AggregationScope; owner_user_id?: string } = {},
): Promise<Snapshot[]> {
  return request<Snapshot[]>(`/api/v1/networth/snapshots${toQuery(params)}`);
}
