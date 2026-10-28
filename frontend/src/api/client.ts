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
export type AuthStatus = components["schemas"]["AuthStatus"];
export type User = components["schemas"]["User"];
export type UserRole = components["schemas"]["UserRole"];
export type Session = components["schemas"]["Session"];
export type Account = components["schemas"]["Account"];
export type AccountType = components["schemas"]["AccountType"];
export type OwnershipScope = components["schemas"]["OwnershipScope"];
export type Balance = components["schemas"]["Balance"];
export type BalanceSource = components["schemas"]["BalanceSource"];
export type CreateAccountRequest = components["schemas"]["CreateAccountRequest"];
export type RecordBalanceRequest = components["schemas"]["RecordBalanceRequest"];

const TOKEN_KEY = "networth.token";
let authToken: string | null = null;
try {
  authToken = localStorage.getItem(TOKEN_KEY);
} catch {
  authToken = null;
}

/** Store or clear the bearer token used for authenticated requests. */
export function setAuthToken(token: string | null): void {
  authToken = token;
  try {
    if (token) {
      localStorage.setItem(TOKEN_KEY, token);
    } else {
      localStorage.removeItem(TOKEN_KEY);
    }
  } catch {
    // ignore storage failures (private mode, tests)
  }
}

/** The current bearer token, if any. */
export function getAuthToken(): string | null {
  return authToken;
}

function authHeaders(): Record<string, string> {
  return authToken ? { authorization: `Bearer ${authToken}` } : {};
}

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
    headers: { "content-type": "application/json", ...authHeaders() },
    ...init,
  });

  if (response.status === 401) {
    setAuthToken(null);
    window.dispatchEvent(new Event("networth:unauthorized"));
  }

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

export type BatchSave = components["schemas"]["BatchSave"];
export type BatchBalanceRow = components["schemas"]["BatchBalanceRow"];

export function recordBalancesBatch(
  id: string,
  rows: BatchBalanceRow[],
  mode: "atomic" | "partial" = "partial",
): Promise<BatchSave> {
  return request<BatchSave>(`/api/v1/accounts/${id}/balances/batch`, {
    method: "POST",
    body: JSON.stringify({ rows, mode }),
  });
}

export type ImportReport = components["schemas"]["ImportReport"];

async function readFileBytes(file: File): Promise<ArrayBuffer> {
  if (typeof file.arrayBuffer === "function") {
    return file.arrayBuffer();
  }
  // Some DOM implementations (notably in tests) lack Blob.arrayBuffer.
  return await new Promise<ArrayBuffer>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as ArrayBuffer);
    reader.onerror = () =>
      reject(reader.error ?? new Error("could not read the file"));
    reader.readAsArrayBuffer(file);
  });
}

/** Upload a CSV or XLSX file; the format is inferred from the extension. */
export async function importSpreadsheet(file: File): Promise<ImportReport> {
  const isXlsx = file.name.toLowerCase().endsWith(".xlsx");
  const format = isXlsx ? "xlsx" : "csv";
  const contentType = isXlsx
    ? "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    : "text/csv";

  const response = await fetch(`${API_BASE_URL}/api/v1/import?format=${format}`, {
    method: "POST",
    headers: { "content-type": contentType, ...authHeaders() },
    // Read the bytes explicitly: this works identically in browsers and in
    // Node-based tests, where a DOM File is not a valid BodyInit.
    body: await readFileBytes(file),
  });

  if (response.status === 401) {
    setAuthToken(null);
    window.dispatchEvent(new Event("networth:unauthorized"));
  }

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

  return (await response.json()) as ImportReport;
}

export function exportUrl(format: "csv" | "xlsx"): string {
  return `${API_BASE_URL}/api/v1/export?format=${format}`;
}

export function templateUrl(format: "csv" | "xlsx"): string {
  return `${API_BASE_URL}/api/v1/template?format=${format}`;
}

export async function getAuthStatus(): Promise<AuthStatus> {
  return request<AuthStatus>("/api/v1/auth/status");
}

export async function setupOwner(body: {
  username: string;
  password: string;
  display_name?: string;
}): Promise<Session> {
  return request<Session>("/api/v1/auth/setup", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function login(body: {
  username: string;
  password: string;
}): Promise<Session> {
  return request<Session>("/api/v1/auth/login", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function logout(): Promise<void> {
  return request<void>("/api/v1/auth/logout", { method: "POST" });
}

export async function getMe(): Promise<User> {
  return request<User>("/api/v1/auth/me");
}

export async function listUsers(): Promise<User[]> {
  return request<User[]>("/api/v1/users");
}

export async function createUser(body: {
  username: string;
  password: string;
  display_name?: string | null;
  role: UserRole;
}): Promise<User> {
  return request<User>("/api/v1/users", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export type Security = components["schemas"]["Security"];
export type SecurityAssetClass = components["schemas"]["SecurityAssetClass"];
export type Sector = components["schemas"]["Sector"];
export type CreateSecurityRequest = components["schemas"]["CreateSecurityRequest"];
export type Holding = components["schemas"]["Holding"];
export type CreateHoldingRequest = components["schemas"]["CreateHoldingRequest"];
export type LotRequest = components["schemas"]["LotRequest"];
export type HoldingBatchRow = components["schemas"]["HoldingBatchRow"];
export type HoldingBatchResult = components["schemas"]["HoldingBatchResult"];
export type Price = components["schemas"]["Price"];
export type ProviderStatus = components["schemas"]["ProviderStatus"];

export const SECURITY_ASSET_CLASSES: SecurityAssetClass[] = [
  "equity",
  "fixed_income",
  "real_estate",
  "commodity",
  "cash",
  "other",
];

export const SECTORS: Sector[] = [
  "energy",
  "materials",
  "industrials",
  "consumer_discretionary",
  "consumer_staples",
  "health_care",
  "financials",
  "information_technology",
  "communication_services",
  "utilities",
  "real_estate",
  "other",
];

export function listSecurities(): Promise<Security[]> {
  return request<Security[]>("/api/v1/securities");
}

export function createSecurity(body: CreateSecurityRequest): Promise<Security> {
  return request<Security>("/api/v1/securities", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function listHoldings(accountId: string): Promise<Holding[]> {
  return request<Holding[]>(`/api/v1/accounts/${accountId}/holdings`);
}

export function createHolding(
  accountId: string,
  body: CreateHoldingRequest,
): Promise<Holding> {
  return request<Holding>(`/api/v1/accounts/${accountId}/holdings`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function deleteHolding(
  accountId: string,
  holdingId: string,
): Promise<void> {
  return request<void>(
    `/api/v1/accounts/${accountId}/holdings/${holdingId}`,
    { method: "DELETE" },
  );
}

export function recordHoldingsBatch(
  accountId: string,
  rows: HoldingBatchRow[],
): Promise<HoldingBatchResult> {
  return request<HoldingBatchResult>(
    `/api/v1/accounts/${accountId}/holdings/batch`,
    { method: "POST", body: JSON.stringify({ rows }) },
  );
}

export function getPriceProvider(): Promise<ProviderStatus> {
  return request<ProviderStatus>("/api/v1/prices/provider");
}

export function getPrice(symbol: string): Promise<Price> {
  return request<Price>(`/api/v1/prices/${encodeURIComponent(symbol)}`);
}

export function setPrice(body: {
  symbol: string;
  price: string;
  currency?: string | null;
}): Promise<Price> {
  return request<Price>("/api/v1/prices", {
    method: "POST",
    body: JSON.stringify(body),
  });
}
