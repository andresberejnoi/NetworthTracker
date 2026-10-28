import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  SECURITY_ASSET_CLASSES,
  SECTORS,
  createHolding,
  createSecurity,
  deleteHolding,
  getPriceProvider,
  listAccounts,
  listHoldings,
  listSecurities,
  recordHoldingsBatch,
  setPrice,
  type SecurityAssetClass,
  type Sector,
} from "../../api/client";
import BulkGrid, {
  type GridColumn,
  type GridRow,
  type SaveResult,
} from "../bulk-entry/BulkGrid";

const INVESTMENT_TYPES = new Set([
  "401k",
  "traditional_ira",
  "roth_ira",
  "taxable_brokerage",
]);

function formatAmount(amount: string | null | undefined, currency: string): string {
  if (amount === null || amount === undefined) {
    return "—";
  }
  const [whole, fraction] = amount.split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${fraction !== undefined ? `${grouped}.${fraction}` : grouped} ${currency}`;
}

const BULK_COLUMNS: GridColumn[] = [
  {
    key: "symbol",
    label: "Symbol",
    validate: (value) => (value === "" ? "symbol is required" : null),
  },
  {
    key: "quantity",
    label: "Quantity",
    validate: (value) => (value === "" ? "quantity is required" : null),
  },
  {
    key: "cost_basis",
    label: "Cost basis",
    validate: (value) => (value === "" ? "cost basis is required" : null),
  },
  {
    key: "acquired_on",
    label: "Acquired",
    placeholder: "YYYY-MM-DD",
    validate: (value) => (value === "" ? "date is required" : null),
  },
];

export default function HoldingsView() {
  const queryClient = useQueryClient();
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";

  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => listAccounts(),
  });
  const investmentAccounts = (accounts.data ?? []).filter((account) =>
    INVESTMENT_TYPES.has(account.account_type),
  );

  const [accountId, setAccountId] = useState("");
  const selectedAccount = accountId || investmentAccounts[0]?.id || "";
  const account = investmentAccounts.find((item) => item.id === selectedAccount);
  const currency = account?.currency ?? "USD";

  const holdings = useQuery({
    queryKey: ["holdings", selectedAccount],
    queryFn: () => listHoldings(selectedAccount),
    enabled: Boolean(selectedAccount),
  });
  const securities = useQuery({
    queryKey: ["securities"],
    queryFn: listSecurities,
  });
  const provider = useQuery({
    queryKey: ["price-provider"],
    queryFn: getPriceProvider,
  });

  const [securitySymbol, setSecuritySymbol] = useState("");
  const [securityName, setSecurityName] = useState("");
  const [assetClass, setAssetClass] = useState<SecurityAssetClass>("equity");
  const [sector, setSector] = useState<Sector>("other");

  const [holdingSymbol, setHoldingSymbol] = useState("");
  const [quantity, setQuantity] = useState("");
  const [costBasis, setCostBasis] = useState("");
  const [acquiredOn, setAcquiredOn] = useState("");

  const [priceSymbol, setPriceSymbol] = useState("");
  const [priceValue, setPriceValue] = useState("");

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["securities"] });
    queryClient.invalidateQueries({ queryKey: ["holdings"] });
    queryClient.invalidateQueries({ queryKey: ["networth"] });
  };

  const addSecurity = useMutation({
    mutationFn: () =>
      createSecurity({
        symbol: securitySymbol,
        name: securityName,
        asset_class: assetClass,
        sector,
      }),
    onSuccess: () => {
      setSecuritySymbol("");
      setSecurityName("");
      refresh();
    },
  });

  const addHolding = useMutation({
    mutationFn: () =>
      createHolding(selectedAccount, {
        symbol: holdingSymbol,
        lots: [{ acquired_on: acquiredOn, quantity, cost_basis: costBasis }],
      }),
    onSuccess: () => {
      setQuantity("");
      setCostBasis("");
      setAcquiredOn("");
      refresh();
    },
  });

  const savePrice = useMutation({
    mutationFn: () => setPrice({ symbol: priceSymbol, price: priceValue }),
    onSuccess: () => {
      setPriceValue("");
      refresh();
    },
  });

  const removeHolding = useMutation({
    mutationFn: (holdingId: string) => deleteHolding(selectedAccount, holdingId),
    onSuccess: refresh,
  });

  const handleBulk = async (rows: GridRow[]): Promise<SaveResult> => {
    const result = await recordHoldingsBatch(
      selectedAccount,
      rows.map((row) => ({
        symbol: row.symbol,
        acquired_on: row.acquired_on,
        quantity: row.quantity,
        cost_basis: row.cost_basis,
      })),
    );
    refresh();
    return { savedCount: result.saved, errors: result.errors };
  };

  const allocation = useMemo(() => {
    const byClass = new Map<string, number>();
    const bySector = new Map<string, number>();

    for (const holding of holdings.data ?? []) {
      const value = Number(holding.market_value ?? "0");
      if (!value) continue;
      byClass.set(holding.asset_class, (byClass.get(holding.asset_class) ?? 0) + value);
      const key = holding.sector ?? "unclassified";
      bySector.set(key, (bySector.get(key) ?? 0) + value);
    }

    return { byClass: [...byClass.entries()], bySector: [...bySector.entries()] };
  }, [holdings.data]);

  return (
    <section aria-label="investments">
      <section className="card" aria-label="holdings">
        <h2>Investments</h2>
        <p className="muted">
          Securities and tax lots inside an investment account. When every
          holding has a price, the account's holdings value replaces its recorded
          balance in net worth.
        </p>

        {accounts.isPending && <p className="muted">Loading accounts …</p>}
        {accounts.data && investmentAccounts.length === 0 && (
          <p className="muted">
            Create an investment account (401k, IRA, Roth IRA, or taxable
            brokerage) first.
          </p>
        )}

        {investmentAccounts.length > 0 && (
          <>
            <div className="controls">
              <label>
                <span>Account</span>
                <select
                  aria-label="holdings account"
                  value={selectedAccount}
                  onChange={(event) => setAccountId(event.target.value)}
                >
                  {investmentAccounts.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </label>
              <span className="badge">
                {provider.data?.configured
                  ? `price add-on: ${provider.data.name}`
                  : "manual prices only"}
              </span>
            </div>

            {holdings.isPending && <p className="muted">Loading holdings …</p>}
            {holdings.isError && (
              <p className="error" role="alert">
                {(holdings.error as Error).message}
              </p>
            )}

            {holdings.data && holdings.data.length === 0 && (
              <p className="muted">
                No holdings yet. Add a security, then record a position.
              </p>
            )}

            {holdings.data && holdings.data.length > 0 && (
              <table className="table">
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>Quantity</th>
                    <th>Cost basis</th>
                    <th>Price</th>
                    <th>Value</th>
                    <th>Unrealized</th>
                    <th aria-label="actions" />
                  </tr>
                </thead>
                <tbody>
                  {holdings.data.map((holding) => (
                    <tr key={holding.id}>
                      <td>
                        {holding.symbol}
                        <span className="muted"> · {holding.asset_class}</span>
                      </td>
                      <td>{holding.quantity}</td>
                      <td>{formatAmount(holding.cost_basis, currency)}</td>
                      <td>
                        {formatAmount(holding.price, currency)}
                        {holding.price_stale && (
                          <span className="badge badge-warn"> stale</span>
                        )}
                      </td>
                      <td>{formatAmount(holding.market_value, currency)}</td>
                      <td>{formatAmount(holding.unrealized_gain, currency)}</td>
                      <td>
                        <button
                          type="button"
                          aria-label={`delete ${holding.symbol}`}
                          onClick={() => removeHolding.mutate(holding.id)}
                          disabled={!canWrite}
                        >
                          ×
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}

            <div className="allocation">
              <section>
                <h3>By asset class</h3>
                <table className="table">
                  <tbody>
                    {allocation.byClass.map(([key, value]) => (
                      <tr key={key}>
                        <td>{key}</td>
                        <td>{formatAmount(String(value), currency)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </section>
              <section>
                <h3>By sector</h3>
                <table className="table">
                  <tbody>
                    {allocation.bySector.map(([key, value]) => (
                      <tr key={key}>
                        <td>{key}</td>
                        <td>{formatAmount(String(value), currency)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </section>
            </div>
          </>
        )}
      </section>

      {investmentAccounts.length > 0 && (
        <>
          <section className="card" aria-label="add security">
            <h2>Add security</h2>
            <form
              className="row"
              onSubmit={(event) => {
                event.preventDefault();
                addSecurity.mutate();
              }}
            >
              <label>
                <span>Symbol</span>
                <input
                  aria-label="security symbol"
                  value={securitySymbol}
                  onChange={(event) => setSecuritySymbol(event.target.value)}
                  placeholder="VTI"
                />
              </label>
              <label>
                <span>Name</span>
                <input
                  aria-label="security name"
                  value={securityName}
                  onChange={(event) => setSecurityName(event.target.value)}
                />
              </label>
              <label>
                <span>Asset class</span>
                <select
                  aria-label="security asset class"
                  value={assetClass}
                  onChange={(event) =>
                    setAssetClass(event.target.value as SecurityAssetClass)
                  }
                >
                  {SECURITY_ASSET_CLASSES.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Sector</span>
                <select
                  aria-label="security sector"
                  value={sector}
                  onChange={(event) => setSector(event.target.value as Sector)}
                >
                  {SECTORS.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              </label>
              <button
                type="submit"
                disabled={!canWrite || addSecurity.isPending}
              >
                Add security
              </button>
            </form>
            {addSecurity.isError && (
              <p className="error" role="alert">
                {(addSecurity.error as Error).message}
              </p>
            )}
          </section>

          <section className="card" aria-label="add holding">
            <h2>Add holding</h2>
            <form
              className="row"
              onSubmit={(event) => {
                event.preventDefault();
                addHolding.mutate();
              }}
            >
              <label>
                <span>Security</span>
                <select
                  aria-label="holding security"
                  value={holdingSymbol}
                  onChange={(event) => setHoldingSymbol(event.target.value)}
                >
                  <option value="">Select …</option>
                  {(securities.data ?? []).map((security) => (
                    <option key={security.id} value={security.symbol}>
                      {security.symbol} — {security.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Quantity</span>
                <input
                  aria-label="holding quantity"
                  value={quantity}
                  onChange={(event) => setQuantity(event.target.value)}
                />
              </label>
              <label>
                <span>Cost basis</span>
                <input
                  aria-label="holding cost basis"
                  value={costBasis}
                  onChange={(event) => setCostBasis(event.target.value)}
                />
              </label>
              <label>
                <span>Acquired</span>
                <input
                  aria-label="holding acquired date"
                  placeholder="YYYY-MM-DD"
                  value={acquiredOn}
                  onChange={(event) => setAcquiredOn(event.target.value)}
                />
              </label>
              <button type="submit" disabled={!canWrite || addHolding.isPending}>
                Add holding
              </button>
            </form>
            {addHolding.isError && (
              <p className="error" role="alert">
                {(addHolding.error as Error).message}
              </p>
            )}
          </section>

          <section className="card" aria-label="manual price">
            <h2>Manual price</h2>
            <form
              className="row"
              onSubmit={(event) => {
                event.preventDefault();
                savePrice.mutate();
              }}
            >
              <label>
                <span>Security</span>
                <select
                  aria-label="price security"
                  value={priceSymbol}
                  onChange={(event) => setPriceSymbol(event.target.value)}
                >
                  <option value="">Select …</option>
                  {(securities.data ?? []).map((security) => (
                    <option key={security.id} value={security.symbol}>
                      {security.symbol}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>Price</span>
                <input
                  aria-label="manual price"
                  value={priceValue}
                  onChange={(event) => setPriceValue(event.target.value)}
                />
              </label>
              <button type="submit" disabled={!canWrite || savePrice.isPending}>
                Save price
              </button>
            </form>
            {savePrice.isError && (
              <p className="error" role="alert">
                {(savePrice.error as Error).message}
              </p>
            )}
          </section>

          <section className="card" aria-label="holdings bulk entry">
            <h2>Bulk holdings entry</h2>
            <p className="muted">
              Paste lots from a spreadsheet. Each row is one lot; rows with the
              same symbol are grouped into one holding.
            </p>
            <BulkGrid
              columns={BULK_COLUMNS}
              minRows={1}
              onSave={handleBulk}
              saveLabel="Save lots"
              readOnly={!canWrite}
            />
          </section>
        </>
      )}
    </section>
  );
}
