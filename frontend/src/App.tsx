import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  API_BASE_URL,
  getAuthStatus,
  getMe,
  getPing,
  logout,
  setAuthToken,
  type AuthStatus,
  type User,
} from "./api/client";
import { AuthProvider } from "./auth/AuthContext";
import AccountsView from "./features/accounts/AccountsView";
import AuthScreen from "./features/auth/AuthScreen";
import BulkEntryView from "./features/bulk-entry/BulkEntryView";
import DataView from "./features/data/DataView";
import NetWorthView from "./features/networth/NetWorthView";
import UsersView from "./features/users/UsersView";

type View = "overview" | "networth" | "accounts" | "bulk" | "data" | "users";

export default function App() {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [authError, setAuthError] = useState<string | null>(null);
  const [view, setView] = useState<View>("overview");

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const authStatus = await getAuthStatus();
        if (cancelled) {
          return;
        }
        setStatus(authStatus);

        // Auth disabled: the backend returns an implicit local owner.
        // Auth enabled with users: resume the stored session.
        if (!authStatus.auth_enabled || !authStatus.needs_setup) {
          try {
            const me = await getMe();
            if (!cancelled) {
              setUser(me);
            }
          } catch {
            setAuthToken(null);
          }
        }
      } catch (error) {
        if (!cancelled) {
          setAuthError((error as Error).message);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void load();

    const onUnauthorized = () => setUser(null);
    window.addEventListener("networth:unauthorized", onUnauthorized);
    return () => {
      cancelled = true;
      window.removeEventListener("networth:unauthorized", onUnauthorized);
    };
  }, []);

  const { data, isPending, isError, error } = useQuery({
    queryKey: ["ping"],
    queryFn: ({ signal }) => getPing(signal),
    enabled: Boolean(user),
  });

  const handleLogout = async () => {
    try {
      await logout();
    } finally {
      setAuthToken(null);
      setUser(null);
    }
  };

  if (loading) {
    return (
      <main className="app">
        <p className="muted">Loading …</p>
      </main>
    );
  }

  if (authError) {
    return (
      <main className="app">
        <p className="error" role="alert">
          {authError}
        </p>
      </main>
    );
  }

  if (status?.auth_enabled && !user) {
    return (
      <AuthScreen
        mode={status.needs_setup ? "setup" : "login"}
        onAuthenticated={(authenticated) => {
          setUser(authenticated);
          setStatus((current) =>
            current ? { ...current, needs_setup: false } : current,
          );
        }}
      />
    );
  }

  if (!user) {
    return (
      <main className="app">
        <p className="error" role="alert">
          Could not determine the current user.
        </p>
      </main>
    );
  }

  return (
    <AuthProvider user={user}>
      <main className="app">
        <header className="hero">
          <div className="hero-top">
            <div>
              <h1>NetworthTracker</h1>
              <p className="subtitle">Local-first net worth tracking</p>
            </div>
            <div className="session">
              <span className="badge">
                {user.display_name} · {user.role}
              </span>
              {status?.auth_enabled && (
                <button type="button" onClick={handleLogout}>
                  Sign out
                </button>
              )}
            </div>
          </div>

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
            <button
              type="button"
              aria-pressed={view === "data"}
              onClick={() => setView("data")}
            >
              Data
            </button>
            {user.role === "owner" && (
              <button
                type="button"
                aria-pressed={view === "users"}
                onClick={() => setView("users")}
              >
                Users
              </button>
            )}
          </nav>
        </header>

        {view === "accounts" ? (
          <AccountsView />
        ) : view === "networth" ? (
          <NetWorthView />
        ) : view === "bulk" ? (
          <BulkEntryView />
        ) : view === "data" ? (
          <DataView />
        ) : view === "users" ? (
          <UsersView />
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
    </AuthProvider>
  );
}
