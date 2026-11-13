import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useCurrentUser } from "../../auth/AuthContext";
import {
  createSnapshot,
  getNetWorth,
  listAccounts,
  listSnapshots,
  type Snapshot,
} from "../../api/client";
import AddBalanceDrawer from "../../components/AddBalanceDrawer";
import Drawer from "../../components/Drawer";
import EmptyState from "../../components/EmptyState";
import PageHeader from "../../components/PageHeader";

type Kind = "total" | "liquid" | "liquidity" | "accessible";
type GroupBy = "by_asset_class" | "by_account_type";

const KINDS: Kind[] = ["total", "liquid", "liquidity", "accessible"];

const KIND_LABELS: Record<Kind, string> = {
  total: "Total",
  liquid: "Liquid",
  liquidity: "Liquidity",
  accessible: "Accessible",
};

const KIND_HINTS: Record<Kind, string> = {
  total: "assets minus liabilities",
  liquid: "cash, brokerage, retirement, and crypto",
  liquidity: "cash available right now",
  accessible: "after estimated taxes, penalties, and costs",
};

const GROUP_LABELS: Record<GroupBy, string> = {
  by_asset_class: "Asset class",
  by_account_type: "Account type",
};

/** Delay a rapidly changing value so it only settles once the user pauses. */
function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delayMs);
    return () => window.clearTimeout(timer);
  }, [value, delayMs]);

  return debounced;
}

/**
 * Format for display only. The exact decimal string returned by the API stays
 * the source of truth; this only adds thousands separators.
 */
function formatAmount(amount: string, currency: string): string {
  const [whole, fraction] = amount.split(".");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${fraction !== undefined ? `${grouped}.${fraction}` : grouped} ${currency}`;
}

/** Exact display subtraction for snapshot values; never uses floating point. */
function decimalSubtract(latest: string, previous: string): string {
  const parse = (value: string) => {
    const negative = value.trim().startsWith("-");
    const [whole, fraction = ""] = value.trim().replace(/^[-+]/, "").split(".");
    return { negative, whole, fraction };
  };
  const left = parse(latest);
  const right = parse(previous);
  const scale = Math.max(left.fraction.length, right.fraction.length);
  const toInt = (part: { negative: boolean; whole: string; fraction: string }) => {
    const digits = `${part.whole}${part.fraction.padEnd(scale, "0")}` || "0";
    const value = BigInt(digits);
    return part.negative ? -value : value;
  };
  let difference = toInt(left) - toInt(right);
  const negative = difference < 0n;
  if (negative) difference = -difference;
  const text = difference.toString().padStart(scale + 1, "0");
  const whole = text.slice(0, text.length - scale) || "0";
  const fraction = scale > 0 ? `.${text.slice(text.length - scale)}` : "";
  return `${negative ? "-" : ""}${whole}${fraction}`;
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
  const currentUser = useCurrentUser();
  const canWrite = currentUser?.role !== "viewer";
  const [ownerAgeInput, setOwnerAgeInput] = useState("35");
  const parsedAge = ownerAgeInput.trim() === "" ? 35 : Number(ownerAgeInput);
  const ownerAge = useDebouncedValue(
    Number.isFinite(parsedAge) ? parsedAge : 35,
    350,
  );
  const [kind, setKind] = useState<Kind>("total");
  const [groupBy, setGroupBy] = useState<GroupBy>("by_asset_class");
  const [assumptionsOpen, setAssumptionsOpen] = useState(false);
  const [balanceOpen, setBalanceOpen] = useState(false);

  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => listAccounts(),
  });

  const networth = useQuery({
    queryKey: ["networth", ownerAge],
    queryFn: () => getNetWorth({ owner_age: ownerAge }),
    // Keep the last figures on screen while a new age is computed, instead of
    // flashing placeholders and shifting the layout.
    placeholderData: keepPreviousData,
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
  const snapshotList = snapshots.data ?? [];
  const latestSnapshot = snapshotList.at(-1);
  const previousSnapshot = snapshotList.at(-2);
  const periodChange =
    latestSnapshot && previousSnapshot
      ? decimalSubtract(latestSnapshot.total, previousSnapshot.total)
      : null;

  return (
    <section aria-label="net worth">
      <PageHeader
        title="Home"
        description="Your household's financial position at a glance."
        actions={
          <>
            <button type="button" onClick={() => setBalanceOpen(true)} disabled={!canWrite}>
              Add balance
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => takeSnapshot.mutate()}
              disabled={takeSnapshot.isPending || !canWrite}
            >
              Take snapshot
            </button>
            <Link className="button-link secondary" to="/data/import-export">
              Import data
            </Link>
            <button
              type="button"
              className="quiet"
              onClick={() => setAssumptionsOpen(true)}
            >
              Assumptions
            </button>
          </>
        }
      />

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

      <p className="muted dashboard-change">
        {periodChange !== null && data
          ? `Change since previous snapshot: ${formatAmount(periodChange, data.currency)}`
          : "Take at least two snapshots to see a period change."}
      </p>

      {networth.isError && (
        <p className="error" role="alert">
          {(networth.error as Error).message}
        </p>
      )}

      <section className="card" aria-label="accounts summary">
        <div className="section-heading">
          <div>
            <h2>Accounts</h2>
            <p className="muted">
              {accounts.isPending
                ? "Loading accounts…"
                : `${accounts.data?.length ?? 0} account${
                    accounts.data?.length === 1 ? "" : "s"
                  } tracked`}
            </p>
          </div>
          <Link to="/accounts">Manage accounts</Link>
        </div>
        {data?.has_stale_data && (
          <p className="notice notice-warning">
            Some balances are stale or manually entered. Review them in Accounts.
          </p>
        )}
        {accounts.data?.length === 0 && (
          <EmptyState
            title="No accounts yet"
            description="Add your first checking, savings, brokerage, or property account."
            action={<Link className="button-link" to="/accounts">Add an account</Link>}
          />
        )}
      </section>

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

      <Drawer
        open={assumptionsOpen}
        title="Planning assumptions"
        onClose={() => setAssumptionsOpen(false)}
      >
        <form className="stack" onSubmit={(event) => event.preventDefault()}>
          <label className="field">
            <span>Owner age</span>
            <input
              aria-label="owner age"
              type="number"
              min={0}
              value={ownerAgeInput}
              onChange={(event) => setOwnerAgeInput(event.target.value)}
            />
          </label>
          <p className="muted">
            Age changes the estimated taxes, penalties, and accessible net worth.
            Results update after you close this panel.
          </p>
          <button type="button" onClick={() => setAssumptionsOpen(false)}>
            Done
          </button>
        </form>
      </Drawer>

      <AddBalanceDrawer
        open={balanceOpen}
        onClose={() => setBalanceOpen(false)}
      />
    </section>
  );
}
