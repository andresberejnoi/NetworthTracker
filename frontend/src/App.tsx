import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router-dom";
import {
  exchangeOidcCode,
  getAuthStatus,
  getMe,
  logout,
  setAuthToken,
  type AuthStatus,
  type Session,
  type User,
} from "./api/client";
import { AuthProvider, useCurrentUser } from "./auth/AuthContext";
import AppShell from "./app/AppShell";
import DataLayout from "./app/DataLayout";
import NotFound from "./app/NotFound";
import PortfolioLayout from "./app/PortfolioLayout";
import SettingsLayout from "./app/SettingsLayout";
import AccountDetailView from "./features/accounts/AccountDetailView";
import AccountsBulkPage from "./features/accounts/AccountsBulkPage";
import AccountsView from "./features/accounts/AccountsView";
import AuthScreen from "./features/auth/AuthScreen";
import BudgetPage from "./features/budget/BudgetPage";
import AggregationView from "./features/aggregation/AggregationView";
import CryptoView from "./features/crypto/CryptoView";
import DataView from "./features/data/DataView";
import MigrationView from "./features/data/MigrationView";
import DocumentsView from "./features/documents/DocumentsView";
import HoldingsView from "./features/investments/HoldingsView";
import NetWorthView from "./features/networth/NetWorthView";
import AboutView from "./features/settings/AboutView";
import BackupsView from "./features/settings/BackupsView";
import SsoSettings from "./features/settings/SsoSettings";
import UsersView from "./features/users/UsersView";

function OwnerOnly({ children }: { children: ReactNode }) {
  const user = useCurrentUser();
  return user?.role === "owner" ? <>{children}</> : <Navigate to="/" replace />;
}

export default function App() {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [authError, setAuthError] = useState<string | null>(null);
  const location = useLocation();
  const navigate = useNavigate();
  const callbackSession = useRef<Promise<Session> | null>(null);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const authStatus = await getAuthStatus();
        if (cancelled) return;
        setStatus(authStatus);

        const callback = location.pathname === "/auth/oidc/callback";
        const params = new URLSearchParams(location.search);
        const ssoCode = params.get("code");
        if (callback && ssoCode) {
          callbackSession.current ??= exchangeOidcCode(ssoCode);
          const session = await callbackSession.current;
          setAuthToken(session.token);
          if (!cancelled) {
            setUser(session.user);
            navigate("/", { replace: true });
          }
          return;
        }
        if (callback && params.get("linked") === "1") {
          navigate("/settings/users", { replace: true });
        }

        if (!authStatus.auth_enabled || !authStatus.needs_setup) {
          try {
            const me = await getMe();
            if (!cancelled) setUser(me);
          } catch {
            setAuthToken(null);
          }
        }
      } catch (error) {
        if (!cancelled) setAuthError((error as Error).message);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void load();
    return () => {
      cancelled = true;
    };
    // The OIDC callback is handled once on initial load.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleLogout = async () => {
    try {
      await logout();
    } finally {
      setAuthToken(null);
      setUser(null);
      navigate("/", { replace: true });
    }
  };

  if (loading) {
    return (
      <main className="app-loading">
        <p className="muted">Loading NetworthTracker…</p>
      </main>
    );
  }

  if (authError) {
    return (
      <main className="app-loading">
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
        oidcEnabled={status.oidc_enabled}
        oidcButtonLabel={status.oidc_button_label}
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
      <main className="app-loading">
        <p className="error" role="alert">
          Could not determine the current user.
        </p>
      </main>
    );
  }

  return (
    <AuthProvider user={user}>
      <Routes>
        <Route
          element={<AppShell user={user} status={status} onLogout={() => void handleLogout()} />}
        >
          <Route index element={<NetWorthView />} />
          <Route path="accounts" element={<AccountsView />} />
          <Route path="accounts/bulk" element={<AccountsBulkPage />} />
          <Route path="accounts/:id" element={<AccountDetailView />} />

          <Route path="portfolio" element={<PortfolioLayout />}>
            <Route index element={<Navigate to="investments" replace />} />
            <Route path="investments" element={<HoldingsView />} />
            <Route path="crypto" element={<CryptoView />} />
          </Route>

          <Route path="data" element={<DataLayout />}>
            <Route index element={<Navigate to="import-export" replace />} />
            <Route path="import-export" element={<DataView />} />
            <Route
              path="migration"
              element={
                <OwnerOnly>
                  <MigrationView />
                </OwnerOnly>
              }
            />
            <Route
              path="documents"
              element={
                <OwnerOnly>
                  <DocumentsView />
                </OwnerOnly>
              }
            />
          </Route>

          <Route path="budget" element={<BudgetPage />} />

          <Route
            path="settings"
            element={
              <OwnerOnly>
                <SettingsLayout />
              </OwnerOnly>
            }
          >
            <Route index element={<Navigate to="users" replace />} />
            <Route path="users" element={<UsersView />} />
            <Route path="sso" element={<SsoSettings />} />
            <Route path="aggregation" element={<AggregationView />} />
            <Route path="backups" element={<BackupsView />} />
            <Route path="about" element={<AboutView />} />
          </Route>

          <Route path="*" element={<NotFound />} />
        </Route>
      </Routes>
    </AuthProvider>
  );
}
