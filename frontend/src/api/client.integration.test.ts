import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";
import {
  API_BASE_URL,
  createAccount,
  getPing,
  listAccounts,
  listBalances,
  recordBalance,
} from "./client";

/**
 * Cross-boundary integration test.
 *
 * Skipped by default so `npm test` stays hermetic. Run it with
 * `scripts/integration-test.sh`, which starts Postgres and the backend and sets
 * `INTEGRATION_TESTS=1` plus `VITE_API_BASE_URL`.
 */
const enabled = process.env.INTEGRATION_TESTS === "1";
const contractPath = resolve(process.cwd(), "../contracts/openapi.yaml");

interface JsonSchema {
  type?: string;
  $ref?: string;
  required?: string[];
}

interface Operation {
  responses?: Record<
    string,
    { content?: Record<string, { schema?: JsonSchema }> }
  >;
}

interface Contract {
  paths: Record<string, Record<string, Operation>>;
  components: { schemas: Record<string, JsonSchema> };
}

describe.skipIf(!enabled)("cross-boundary integration", () => {
  it("consumes the live backend through the frontend client", async () => {
    const ping = await getPing();
    expect(ping.service).toBe("NetworthTracker");
    expect(ping.message).toBe("pong");
  });

  it("creates an account and records a balance over HTTP", async () => {
    const created = await createAccount({
      name: `Integration ${Date.now()}`,
      account_type: "checking",
      institution: "Integration Bank",
    });
    expect(created.account_type).toBe("checking");
    expect(created.archived).toBe(false);

    const balance = await recordBalance(created.id, {
      as_of: "2026-10-17",
      amount: "1234.56",
    });
    expect(balance.amount).toBe("1234.56");

    const listed = await listAccounts();
    expect(listed.some((account) => account.id === created.id)).toBe(true);

    const balances = await listBalances(created.id);
    expect(balances.some((b) => b.amount === "1234.56")).toBe(true);
  });

  it("matches the shared OpenAPI contract for GET endpoints", async () => {
    const contract = parse(readFileSync(contractPath, "utf8")) as Contract;

    const account = await createAccount({
      name: `Contract ${Date.now()}`,
      account_type: "savings",
    });

    for (const [path, methods] of Object.entries(contract.paths)) {
      const operation = methods.get;
      const schema =
        operation?.responses?.["200"]?.content?.["application/json"]?.schema;
      if (!operation || !schema) continue;

      const concretePath = path.replace("{id}", account.id);
      const response = await fetch(`${API_BASE_URL}${concretePath}`);
      expect(response.status, `GET ${concretePath}`).toBe(200);

      const body = (await response.json()) as unknown;
      const resolved = schema.$ref
        ? contract.components.schemas[schema.$ref.split("/").pop() as string]
        : schema;

      if (resolved.type === "array") {
        expect(Array.isArray(body), `GET ${concretePath} is an array`).toBe(true);
      } else {
        for (const field of resolved.required ?? []) {
          expect(body, `GET ${concretePath}`).toHaveProperty(field);
        }
      }
    }
  });
});
