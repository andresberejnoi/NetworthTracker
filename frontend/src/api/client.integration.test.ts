/**
 * Cross-boundary integration test.
 *
 * This is the walking-skeleton proof that the decoupled frontend really
 * consumes the backend over HTTP. It is skipped by default so `npm test` stays
 * hermetic; run it with `scripts/integration-test.sh` (or set
 * `INTEGRATION_TESTS=1` and `VITE_API_BASE_URL` to a running backend).
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";
import { API_BASE_URL, getPing } from "./client";

const enabled = process.env.INTEGRATION_TESTS === "1";

interface JsonSchema {
  $ref?: string;
  required?: string[];
}

interface Contract {
  paths: Record<
    string,
    Record<
      string,
      { responses?: Record<string, { content?: Record<string, { schema?: JsonSchema }> }> }
    >
  >;
  components: { schemas: Record<string, JsonSchema> };
}

const contractPath = resolve(process.cwd(), "../contracts/openapi.yaml");

describe.skipIf(!enabled)("cross-boundary integration", () => {
  it("consumes the live backend through the frontend client", async () => {
    const ping = await getPing();
    expect(ping.service).toBe("NetworthTracker");
    expect(ping.message).toBe("pong");
  });

  it("matches the shared OpenAPI contract", async () => {
    const contract = parse(
      readFileSync(contractPath, "utf8"),
    ) as Contract;

    for (const [path, methods] of Object.entries(contract.paths)) {
      for (const [method, operation] of Object.entries(methods)) {
        const schema =
          operation.responses?.["200"]?.content?.["application/json"]?.schema;
        if (!schema) continue;

        const response = await fetch(`${API_BASE_URL}${path}`, {
          method: method.toUpperCase(),
        });
        expect(response.status, `${method.toUpperCase()} ${path}`).toBe(200);

        const resolved = schema.$ref
          ? contract.components.schemas[schema.$ref.split("/").pop() as string]
          : schema;
        const body = (await response.json()) as Record<string, unknown>;
        for (const field of resolved.required ?? []) {
          expect(body, `${method.toUpperCase()} ${path}`).toHaveProperty(field);
        }
      }
    }
  });
});
