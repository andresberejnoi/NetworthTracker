import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getAggregation,
  saveAggregationConfig,
  syncAggregation,
} from "../../api/client";

function formatAmount(amount: string | null | undefined, currency: string): string {
  if (amount === null || amount === undefined) {
    return "—";
  }
  const [whole, fraction] = amount.split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${fraction !== undefined ? `${grouped}.${fraction}` : grouped} ${currency}`;
}

export default function AggregationView() {
  const queryClient = useQueryClient();
  const view = useQuery({
    queryKey: ["aggregation"],
    queryFn: getAggregation,
  });

  const [provider, setProvider] = useState("fake");
  const [token, setToken] = useState("");
  const [enabled, setEnabled] = useState(true);

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: ["aggregation"] });
  };

  const saveConfig = useMutation({
    mutationFn: () =>
      saveAggregationConfig({
        provider,
        enabled,
        token: token || null,
      }),
    onSuccess: () => {
      setToken("");
      refresh();
    },
  });

  const sync = useMutation({
    mutationFn: syncAggregation,
    onSuccess: refresh,
  });

  return (
    <section aria-label="aggregation">
      <section className="card" aria-label="aggregation config">
        <h2>Account aggregation</h2>
        <p className="muted">
          Optional read-only linking. Tokens are encrypted locally and sent only
          to the provider you configure; there is no central relay.
        </p>
        <div className="controls">
          <span className="badge">
            {view.data?.status.provider_available
              ? "provider adapter available"
              : "no provider adapter"}
          </span>
          <span className="badge">
            {view.data?.status.enabled ? "enabled" : "disabled"}
          </span>
          {view.data?.status.has_token && <span className="badge">token stored</span>}
        </div>
        <form
          className="row"
          onSubmit={(event) => {
            event.preventDefault();
            saveConfig.mutate();
          }}
        >
          <label>
            <span>Provider</span>
            <select
              aria-label="aggregation provider"
              value={provider}
              onChange={(event) => setProvider(event.target.value)}
            >
              <option value="fake">fake</option>
            </select>
          </label>
          <label>
            <span>Own token / API key</span>
            <input
              aria-label="aggregation token"
              type="password"
              placeholder="leave blank to keep the stored token"
              value={token}
              onChange={(event) => setToken(event.target.value)}
            />
          </label>
          <label>
            <span>Enabled</span>
            <input
              aria-label="aggregation enabled"
              type="checkbox"
              checked={enabled}
              onChange={(event) => setEnabled(event.target.checked)}
            />
          </label>
          <button type="submit" disabled={saveConfig.isPending}>
            Save config
          </button>
          <button
            type="button"
            onClick={() => sync.mutate()}
            disabled={!view.data?.status.enabled || sync.isPending}
          >
            Sync now
          </button>
        </form>
        {saveConfig.isError && (
          <p className="error" role="alert">
            {(saveConfig.error as Error).message}
          </p>
        )}
        {sync.isError && (
          <p className="error" role="alert">
            {(sync.error as Error).message}
          </p>
        )}
      </section>

      <section className="card" aria-label="aggregation links">
        <h2>Linked accounts</h2>
        {view.isPending && <p className="muted">Loading aggregation …</p>}
        {view.isError && (
          <p className="error" role="alert">
            {(view.error as Error).message}
          </p>
        )}
        {view.data && view.data.links.length === 0 && (
          <p className="muted">No linked accounts yet.</p>
        )}
        {view.data && view.data.links.length > 0 && (
          <table className="table">
            <thead>
              <tr>
                <th>Account</th>
                <th>Provider balance</th>
                <th>Local balance</th>
                <th>In effect</th>
                <th>Last sync</th>
              </tr>
            </thead>
            <tbody>
              {view.data.links.map((link) => (
                <tr key={link.id}>
                  <td>
                    {link.display_name}
                    <span className="muted"> · {link.provider}</span>
                  </td>
                  <td>{formatAmount(link.provider_balance, link.currency)}</td>
                  <td>
                    {formatAmount(link.local_balance, link.currency)}
                    {link.local_source && (
                      <span className="muted"> · {link.local_source}</span>
                    )}
                  </td>
                  <td>{link.in_effect}</td>
                  <td>{link.last_synced_at ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {view.data && view.data.links.some((link) => link.holdings.length > 0) && (
          <>
            <h3>Provider holdings</h3>
            <table className="table">
              <thead>
                <tr>
                  <th>Account</th>
                  <th>Symbol</th>
                  <th>Quantity</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                {view.data.links.flatMap((link) =>
                  link.holdings.map((holding) => (
                    <tr key={`${link.id}-${holding.symbol}`}>
                      <td>{link.display_name}</td>
                      <td>{holding.symbol}</td>
                      <td>{holding.quantity}</td>
                      <td>{formatAmount(holding.value, holding.currency)}</td>
                    </tr>
                  )),
                )}
              </tbody>
            </table>
          </>
        )}
      </section>
    </section>
  );
}
