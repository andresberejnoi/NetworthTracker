import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createSnapshot,
  getNetWorth,
  listSnapshots,
  type Snapshot,
} from "../../api/client";

type Kind = "total" | "liquid" | "accessible";
type GroupBy = "by_asset_class" | "by_account_type";

const KINDS: Kind[] = ["total", "liquid", "accessible"];

const KIND_LABELS: Record<Kind, string> = {
  total: "Total",
  liquid: "Liquid",
  accessible: "Accessible",
};

const KIND_HINTS: Record<Kind, string> = {
  total: "assets minus liabilities",
  liquid: "cash-like assets minus debts",
  accessible: "after estimated taxes, penalties, and costs",
};

const GROUP_LABELS: Record<GroupBy, string> = {
  by_asset_class: "Asset class",
  by_account_type: "Account type",
};

/**
 * Format for display only. The exact decimal string returned by the API stays
 * the source of truth; this only adds thousands separators.
 */
function formatAmount(amount: string, currency: string): string {
  const [whole, fraction] = amount.split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${fraction !== undefined ? `${grouped}.${fraction}` : grouped} ${currency}`;
}

function HistoryChart({
  snapshots,
  kind,
}: {
  snapshots: Snapshot[];
  kind: Kind;
}) {
  if (snapshots.length < 2) {
    return (
      <p className="muted">
        Take at least two snapshots to see a trend.
      </p>
    );
  }

  // Numbers here are only for pixel geometry, never for money math.
  const values = snapshots.map((snapshot) => Number(snapshot[kind]));
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const width = 600;
  const height = 160;
  const pad = 12;

  const points = values
    .map((value, index) => {
      const x = pad + (index * (width - pad * 2)) / (values.length - 1);
      const y = height - pad - ((value - min) / span) * (height - pad * 2);
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg
      className="chart"
      viewBox={`0 0 ${width} ${height}`}
      role="img"
      aria-label={`${kind} net worth history`}
    >
      <polyline points={points} fill="none" stroke="var(--accent)" strokeWidth="2" />
    </svg>
  );
}

export default function NetWorthView() {
  const queryClient = useQueryClient();
  const [ownerAge, setOwnerAge] = useState(35);
  const [kind, setKind] = useState<Kind>("total");
  const [groupBy, setGroupBy] = useState<GroupBy>("by_asset_class");

  const networth = useQuery({
    queryKey: ["networth", ownerAge],
    queryFn: () => getNetWorth({ owner_age: ownerAge }),
  });

  const snapshots = useQuery({
    queryKey: ["snapshots"],
    queryFn: () => listSnapshots(),
  });

  const takeSnapshot = useMutation({
    mutationFn: () => createSnapshot({ owner_age: ownerAge }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["snapshots"] }),
  });

  const data = networth.data;

  return (
    <section aria-label="net worth">
      <div className="cards">
        {KINDS.map((metric) => (
          <article className="card metric" key={metric}>
            <h3>{KIND_LABELS[metric]}</h3>
            <p className="metric-value">
              {data ? formatAmount(data[metric], data.currency) : "—"}
            </p>
            <p className="muted">{KIND_HINTS[metric]}</p>
          </article>
        ))}
      </div>

      {networth.isError && (
        <p className="error" role="alert">
          {(networth.error as Error).message}
        </p>
      )}

      <section className="card" aria-label="history">
        <h2>History</h2>
        <div className="controls">
          <label>
            <span>Metric</span>
            <select
              aria-label="history metric"
              value={kind}
              onChange={(event) => setKind(event.target.value as Kind)}
            >
              {KINDS.map((option) => (
                <option key={option} value={option}>
                  {KIND_LABELS[option]}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Owner age</span>
            <input
              aria-label="owner age"
              type="number"
              min={0}
              value={ownerAge}
              onChange={(event) => setOwnerAge(Number(event.target.value))}
            />
          </label>
          <button
            type="button"
            onClick={() => takeSnapshot.mutate()}
            disabled={takeSnapshot.isPending}
          >
            Take snapshot
          </button>
        </div>

        {snapshots.isPending && <p className="muted">Loading history …</p>}
        {snapshots.isError && (
          <p className="error" role="alert">
            {(snapshots.error as Error).message}
          </p>
        )}
        {snapshots.data && (
          <HistoryChart snapshots={snapshots.data} kind={kind} />
        )}
      </section>

      <section className="card" aria-label="breakdown">
        <h2>Breakdown</h2>
        <div className="controls">
          <label>
            <span>Group by</span>
            <select
              aria-label="group by"
              value={groupBy}
              onChange={(event) => setGroupBy(event.target.value as GroupBy)}
            >
              {(Object.keys(GROUP_LABELS) as GroupBy[]).map((option) => (
                <option key={option} value={option}>
                  {GROUP_LABELS[option]}
                </option>
              ))}
            </select>
          </label>
          <div className="badges">
            {data?.has_manual_data && (
              <span className="badge">manual entries</span>
            )}
            {data?.has_stale_data && (
              <span className="badge badge-warn">stale balances</span>
            )}
          </div>
        </div>

        {data && data[groupBy].length === 0 && (
          <p className="muted">
            No balances yet — add accounts and record balances first.
          </p>
        )}

        {data && data[groupBy].length > 0 && (
          <table className="table">
            <thead>
              <tr>
                <th>{GROUP_LABELS[groupBy]}</th>
                <th>Net amount</th>
              </tr>
            </thead>
            <tbody>
              {data[groupBy].map((row) => (
                <tr key={row.key}>
                  <td>{row.key}</td>
                  <td>{formatAmount(row.amount, row.currency)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </section>
  );
}
