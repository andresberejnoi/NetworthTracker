import { useEffect, useRef, useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import type { AuthStatus, User } from "../api/client";
import {
  primaryNavigation,
  secondaryNavigation,
  settingsNavigation,
  titleForPath,
} from "./navigation";

interface AppShellProps {
  user: User;
  status: AuthStatus | null;
  onLogout: () => void;
}

function navClass({ isActive }: { isActive: boolean }): string {
  return isActive ? "nav-item active" : "nav-item";
}

export default function AppShell({ user, status, onLogout }: AppShellProps) {
  const location = useLocation();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const moreButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    setSidebarOpen(false);
    setMoreOpen(false);
  }, [location.pathname]);

  useEffect(() => {
    if (!moreOpen && !sidebarOpen) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (moreOpen) {
        setMoreOpen(false);
        moreButton.current?.focus();
      }
      setSidebarOpen(false);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [moreOpen, sidebarOpen]);

  const isOwner = user.role === "owner";

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>

      <aside
        className={sidebarOpen ? "sidebar open" : "sidebar"}
        aria-label="Primary navigation"
      >
        <div className="brand">
          <span className="brand-mark">NW</span>
          <div>
            <strong>NetworthTracker</strong>
            <span className="brand-subtitle">Local-first finance</span>
          </div>
        </div>

        <nav className="sidebar-nav" aria-label="Main">
          <p className="nav-heading">Main</p>
          {primaryNavigation.map((item) => (
            <NavLink key={item.to} to={item.to} end={item.end} className={navClass}>
              {item.label}
            </NavLink>
          ))}
          {secondaryNavigation
            .filter((item) => !item.ownerOnly || isOwner)
            .map((item) => (
              <NavLink key={item.to} to={item.to} className={navClass}>
                {item.label}
              </NavLink>
            ))}
        </nav>

        {isOwner && (
          <nav className="sidebar-nav sidebar-settings" aria-label="Settings">
            <p className="nav-heading">Settings</p>
            {settingsNavigation.map((item) => (
              <NavLink key={item.to} to={item.to} className={navClass}>
                {item.label}
              </NavLink>
            ))}
          </nav>
        )}
      </aside>

      {sidebarOpen && (
        <button
          type="button"
          className="sidebar-backdrop"
          aria-label="Close navigation"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      <div className="shell-main">
        <header className="topbar">
          <button
            type="button"
            className="sidebar-toggle"
            aria-label="Open navigation"
            aria-expanded={sidebarOpen}
            onClick={() => setSidebarOpen(true)}
          >
            Menu
          </button>
          <span className="topbar-title">{titleForPath(location.pathname)}</span>
          <div className="session">
            <span className="badge">
              {user.display_name} · {user.role}
            </span>
            {status?.auth_enabled && (
              <button type="button" className="quiet" onClick={onLogout}>
                Sign out
              </button>
            )}
          </div>
        </header>

        <main id="main-content" className="content" tabIndex={-1}>
          <Outlet />
        </main>
      </div>

      <nav className="mobile-nav" aria-label="Mobile primary navigation">
        {primaryNavigation.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            className={({ isActive }) =>
              isActive ? "mobile-nav-item active" : "mobile-nav-item"
            }
          >
            {item.label === "Portfolio" ? "Portfolio" : item.label}
          </NavLink>
        ))}
        <button
          ref={moreButton}
          type="button"
          className={moreOpen ? "mobile-nav-item active" : "mobile-nav-item"}
          aria-expanded={moreOpen}
          aria-controls="more-menu"
          onClick={() => setMoreOpen((open) => !open)}
        >
          More
        </button>
      </nav>

      {moreOpen && (
        <div className="more-sheet" id="more-menu" role="dialog" aria-label="More destinations">
          <div className="more-sheet-header">
            <strong>More</strong>
            <button
              type="button"
              className="quiet"
              onClick={() => {
                setMoreOpen(false);
                moreButton.current?.focus();
              }}
            >
              Close
            </button>
          </div>
          <NavLink to="/budget" className={navClass}>
            Budget
          </NavLink>
          {isOwner && (
            <NavLink to="/settings/aggregation" className={navClass}>
              Aggregation
            </NavLink>
          )}
          {isOwner && (
            <NavLink to="/settings/users" className={navClass}>
              Settings
            </NavLink>
          )}
        </div>
      )}
    </div>
  );
}
