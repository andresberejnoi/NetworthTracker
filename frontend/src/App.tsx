import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { API_BASE_URL, getPing } from "./api/client";
import AccountsView from "./features/accounts/AccountsView";
import BulkEntryView from "./features/bulk-entry/BulkEntryView";
import NetWorthView from "./features/networth/NetWorthView";

type View = "overview" | "networth" | "accounts" | "bulk";

export default function App() {
  const [view, setView] = useState<View>("overview");

  const { data, isPending, isError, error } = useQuery({
    queryKey: ["ping"],
    queryFn: ({ signal }) => getPing(signal),
  });

  return (
    <main className="app">
      <header className="hero">
        <h1>NetworthTracker</h1>
        <p className="subtitle">Local-first net worth tracking</p>
        <nav className="nav" aria-label="views">
          <button
            type="button"
            aria-pressed={view === "overview"}
            onClick={() => setView("overview")}
          >
            Overview
          </button>
          <button
            type="button"
            aria-pressed={view === "networth"}
            onClick={() => setView("networth")}
          >
            Net worth
          </button>
          <button
            type="button"
            aria-pressed={view === "accounts"}
            onClick={() => setView("accounts")}
          >
            Accounts
          </button>
          <button
            type="button"
            aria-pressed={view === "bulk"}
            onClick={() => setView("bulk")}
          >
            Bulk entry
          </button>
        </nav>
      </header>

      {view === "accounts" ? (
        <AccountsView />
      ) : view === "networth" ? (
        <NetWorthView />
      ) : view === "bulk" ? (
        <BulkEntryView />
      ) : (
        <section className="card" aria-label="backend status">
          <h2>Backend</h2>
          {isPending && <p className="muted">Checking {API_BASE_URL} …</p>}
          {isError && (
            <p className="error" role="alert">
              Unreachable: {(error as Error).message}
            </p>
          )}
          {data && (
            <dl className="meta">
              <div>
                <dt>Service</dt>
                <dd>{data.service}</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{data.version}</dd>
              </div>
              <div>
                <dt>Reply</dt>
                <dd>{data.message}</dd>
              </div>
            </dl>
          )}
        </section>
      )}
    </main>
  );
}
