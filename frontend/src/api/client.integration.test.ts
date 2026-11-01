import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";
import {
  API_BASE_URL,
  createAccount,
  createCryptoAsset,
  createCryptoLot,
  createCryptoSale,
  createCryptoWallet,
  createHolding,
  createSale,
  createSecurity,
  createSnapshot,
  getAggregation,
  getBudget,
  getNetWorth,
  getPing,
  listAccounts,
  listBalances,
  listCryptoLots,
  listCryptoPositions,
  listCryptoSales,
  listHoldings,
  listSales,
  listSnapshots,
  importSpreadsheet,
  recordBalance,
  recordBalancesBatch,
  recordBudgetSpending,
  recordCryptoBalance,
  saveAggregationConfig,
  setBudgetIncome,
  setBudgetPlan,
  setPrice,
  syncAggregation,
  syncCryptoWallet,
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

  it("computes net worth and persists an immutable snapshot", async () => {
    const account = await createAccount({
      name: `Net worth ${Date.now()}`,
      account_type: "cash",
    });
    await recordBalance(account.id, { as_of: "2026-10-17", amount: "1000.00" });

    const networth = await getNetWorth({ as_of: "2026-10-17", owner_age: 35 });
    expect(networth.account_count).toBeGreaterThan(0);
    expect(networth.total).toMatch(/^\d+\.\d{2}$/);

    const snapshot = await createSnapshot({ as_of: "2026-10-17", owner_age: 35 });
    expect(snapshot.total).toBe(networth.total);
    expect(snapshot.scope).toBe("household");

    const snapshots = await listSnapshots();
    expect(snapshots.some((item) => item.id === snapshot.id)).toBe(true);
    expect(snapshots[snapshots.length - 1].id).toBe(snapshot.id);
  });

  it("saves a grid of balances in one request", async () => {
    const account = await createAccount({
      name: `Bulk ${Date.now()}`,
      account_type: "checking",
    });

    const result = await recordBalancesBatch(
      account.id,
      [
        { as_of: "2026-01-01", amount: "10.00" },
        { as_of: "2026-02-01", amount: "20.00" },
        { as_of: "2026-03-01", amount: "oops" },
      ],
      "partial",
    );

    expect(result.mode).toBe("partial");
    expect(result.saved).toHaveLength(2);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0].index).toBe(2);

    const balances = await listBalances(account.id);
    expect(balances).toHaveLength(2);
  });

  it("imports a spreadsheet over HTTP and reports bad rows", async () => {
    const name = `Imported ${Date.now()}`;
    const header =
      "schema_version,record_type,account_name,account_type,institution,currency,ownership_scope,archived,as_of,amount,source";
    const csv = `${header}\n1,account,${name},checking,,USD,household,false,,,\n1,balance,${name},,,,,,2026-05-01,321.00,manual\n`;

    const report = await importSpreadsheet(
      new File([csv], "integration.csv", { type: "text/csv" }),
    );
    expect(report.errors).toHaveLength(0);
    expect(report.accounts_created).toBe(1);
    expect(report.balances_saved).toBe(1);

    const bad = `${header}\n1,balance,${name},,,,,,2026-05-02,oops,manual\n`;
    const badReport = await importSpreadsheet(
      new File([bad], "bad.csv", { type: "text/csv" }),
    );
    expect(badReport.balances_saved).toBe(0);
    expect(badReport.errors).toHaveLength(1);
  });

  it("values investment holdings over HTTP", async () => {
    const stamp = Date.now();
    const account = await createAccount({
      name: `Brokerage ${stamp}`,
      account_type: "taxable_brokerage",
    });
    const symbol = `T${stamp}`.slice(0, 12);

    await createSecurity({
      symbol,
      name: "Integration Security",
      asset_class: "equity",
      sector: "other",
    });
    await createHolding(account.id, {
      symbol,
      lots: [
        { acquired_on: "2026-01-01", quantity: "10", cost_basis: "1000.00" },
        { acquired_on: "2026-02-01", quantity: "5", cost_basis: "600.00" },
      ],
    });
    await setPrice({ symbol, price: "150.00" });

    const holdings = await listHoldings(account.id);
    const holding = holdings.find((item) => item.symbol === symbol);
    expect(holding).toBeDefined();
    expect(holding?.quantity).toBe("15");
    expect(holding?.market_value).toBe("2250.00");
    expect(holding?.unrealized_gain).toBe("650.00");
  });

  it("appends lots and sells FIFO over HTTP", async () => {
    const stamp = Date.now();
    const account = await createAccount({
      name: `Tax lots ${stamp}`,
      account_type: "taxable_brokerage",
    });
    const symbol = `L${stamp}`.slice(0, 12);

    await createSecurity({
      symbol,
      name: "Lot Integration",
      asset_class: "equity",
      sector: "other",
    });
    await createHolding(account.id, {
      symbol,
      lots: [
        {
          acquired_on: "2026-01-01",
          quantity: "10",
          cost_basis: "1000.00",
          basis_quality: "actual",
        },
      ],
    });
    // A second append must preserve the first lot.
    await createHolding(account.id, {
      symbol,
      lots: [
        {
          acquired_on: "2026-02-01",
          quantity: "5",
          cost_basis: "600.00",
          basis_quality: "estimated",
          basis_note: "backfilled",
        },
      ],
    });

    const before = (await listHoldings(account.id)).find(
      (item) => item.symbol === symbol,
    );
    expect(before?.quantity).toBe("15");
    expect(before?.lots).toHaveLength(2);

    const sale = await createSale(account.id, {
      symbol,
      disposed_on: "2026-06-01",
      quantity: "12",
      gross_proceeds: "1800.00",
      fees: "12.00",
      long_term_years: 1,
    });
    expect(sale.realized_gain).toBe("548.00");
    expect(sale.disposals).toHaveLength(2);
    expect(sale.disposals[0].quantity).toBe("10");
    expect(sale.disposals[1].basis_quality).toBe("estimated");

    const after = (await listHoldings(account.id)).find(
      (item) => item.symbol === symbol,
    );
    expect(after?.quantity).toBe("3");
    expect(after?.cost_basis).toBe("360.00");

    const sales = await listSales(account.id);
    expect(sales.some((item) => item.id === sale.id)).toBe(true);
  });

  it("tracks manual and address crypto wallets over HTTP", async () => {
    const stamp = Date.now();
    const asset = await createCryptoAsset({
      symbol: "BTC",
      name: `Bitcoin ${stamp}`,
      chain: "bitcoin",
      decimals: 18,
    });
    const manual = await createCryptoWallet({
      asset_id: asset.id,
      kind: "manual",
      chain: "bitcoin",
      label: `Coinbase ${stamp}`,
    });
    await recordCryptoBalance(manual.id, {
      as_of: "2026-01-01",
      quantity: "0.003",
    });
    await setPrice({ symbol: "BTC", price: "70000.00" });

    await createCryptoLot(manual.id, {
      acquired_on: "2026-01-01",
      quantity: "0.003",
      cost_basis: "210.00",
      basis_quality: "actual",
    });
    const lots = await listCryptoLots(manual.id);
    expect(lots).toHaveLength(1);
    expect(lots[0].quantity).toBe("0.003");

    const cryptoSale = await createCryptoSale(manual.id, {
      symbol: "BTC",
      disposed_on: "2026-06-01",
      quantity: "0.001",
      gross_proceeds: "80.00",
      fees: "1.00",
    });
    expect(cryptoSale.realized_gain).toBe("9.00");
    expect((await listCryptoSales(manual.id)).length).toBe(1);

    let positions = await listCryptoPositions({ as_of: "2026-01-01" });
    let btc = positions.find((item) => item.asset_id === asset.id);
    expect(btc?.quantity).toBe("0.003");
    expect(btc?.value).toBe("210.00");

    const address = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
    const addressWallet = await createCryptoWallet({
      asset_id: asset.id,
      kind: "address",
      chain: "bitcoin",
      label: `Cold ${stamp}`,
      address,
    });
    const synced = await syncCryptoWallet(addressWallet.id);
    expect(synced.quantity).toBe("0.000000000000000001");
    expect(synced.source).toBe("chain");

    positions = await listCryptoPositions();
    btc = positions.find((item) => item.asset_id === asset.id);
    expect(btc?.wallet_count).toBe(2);
    expect(btc?.quantity).toBe("0.002000000000000001");

    const networth = await getNetWorth({ as_of: "2026-01-01", owner_age: 35 });
    expect(Number(networth.total)).toBeGreaterThanOrEqual(210);
  });

  it("records income, a spending plan, and variance over HTTP", async () => {
    const year = 2026;
    const empty = await getBudget(year);
    expect(empty.year).toBe(year);

    await setBudgetIncome({ year, amount: "120000.00" });
    await setBudgetPlan({
      year,
      allocations: [
        { category: "fixed_costs", percentage: "50" },
        { category: "savings", percentage: "20" },
        { category: "discretionary", percentage: "30" },
      ],
    });
    await recordBudgetSpending({
      year,
      month: 1,
      category: "fixed_costs",
      amount: "1000.00",
    });

    const budget = await getBudget(year);
    expect(budget.income).toBe("120000.00");
    expect(budget.balanced).toBe(true);
    const fixed = budget.variance.find((row) => row.category === "fixed_costs");
    expect(fixed?.planned).toBe("60000.00");
    expect(fixed?.actual).toBe("1000.00");
    expect(fixed?.variance_amount).toBe("59000.00");
  });

  it("syncs a fake read-only aggregation provider over HTTP", async () => {
    const empty = await getAggregation();
    expect(empty.status.provider_available).toBe(true);

    await saveAggregationConfig({
      provider: "fake",
      enabled: true,
      token: "integration-token",
    });
    const view = await syncAggregation();
    expect(view.status.enabled).toBe(true);
    expect(view.status.has_token).toBe(true);
    expect(view.links).toHaveLength(1);
    expect(view.links[0].display_name).toBe("Fake Brokerage");
    expect(view.links[0].provider_balance).toBe("12345.67");
    expect(view.links[0].holdings[0].symbol).toBe("VTI");

    await recordBalance(view.links[0].account_id, {
      as_of: new Date().toISOString().slice(0, 10),
      amount: "999.00",
    });
    const after = await syncAggregation();
    expect(after.links[0].local_balance).toBe("999.00");
    expect(after.links[0].in_effect).toBe("manual");
  });

  it("matches the shared OpenAPI contract for GET endpoints", async () => {
    const contract = parse(readFileSync(contractPath, "utf8")) as Contract;

    const account = await createAccount({
      name: `Contract ${Date.now()}`,
      account_type: "savings",
    });

    // Values for the dynamic GET endpoints.
    const symbol = `C${Date.now()}`.slice(0, 12);
    await createSecurity({
      symbol,
      name: "Contract Security",
      asset_class: "equity",
      sector: "other",
    });
    await setPrice({ symbol, price: "100.00" });
    const cryptoAsset = await createCryptoAsset({
      symbol: `B${Date.now()}`.slice(0, 12),
      name: "Contract Crypto",
      chain: "bitcoin",
      decimals: 18,
    });
    const cryptoWallet = await createCryptoWallet({
      asset_id: cryptoAsset.id,
      kind: "manual",
      chain: "bitcoin",
      label: "Contract Wallet",
    });
    await fetch(`${API_BASE_URL}/api/v1/fx-rates`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ base: "USD", quote: "EUR", rate: "0.92" }),
    });

    for (const [path, methods] of Object.entries(contract.paths)) {
      const operation = methods.get;
      const schema =
        operation?.responses?.["200"]?.content?.["application/json"]?.schema;
      if (!operation || !schema) continue;

      let concretePath = path
        .replace("{id}", account.id)
        .replace("{symbol}", symbol);
      if (path.startsWith("/api/v1/crypto/wallets/")) {
        concretePath = path.replace("{id}", cryptoWallet.id);
      }
      if (concretePath === "/api/v1/fx-rates") {
        concretePath += "?base=USD&quote=EUR";
      }

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
