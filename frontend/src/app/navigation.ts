export interface NavigationItem {
  to: string;
  label: string;
  ownerOnly?: boolean;
  end?: boolean;
}

export const primaryNavigation: NavigationItem[] = [
  { to: "/", label: "Home", end: true },
  { to: "/accounts", label: "Accounts" },
  { to: "/portfolio/investments", label: "Portfolio" },
  { to: "/data/import-export", label: "Data" },
];

export const secondaryNavigation: NavigationItem[] = [
  { to: "/budget", label: "Budget" },
];

export const settingsNavigation: NavigationItem[] = [
  { to: "/settings/users", label: "Users & access", ownerOnly: true },
  { to: "/settings/sso", label: "SSO", ownerOnly: true },
  { to: "/settings/aggregation", label: "Aggregation", ownerOnly: true },
  { to: "/settings/backups", label: "Backups", ownerOnly: true },
  { to: "/settings/about", label: "About" },
];

const titles: Array<[string, string]> = [
  ["/accounts", "Accounts"],
  ["/portfolio", "Portfolio"],
  ["/data", "Data"],
  ["/budget", "Budget"],
  ["/settings", "Settings"],
];

export function titleForPath(pathname: string): string {
  if (pathname === "/") return "Home";
  return titles.find(([prefix]) => pathname.startsWith(prefix))?.[1] ?? "NetworthTracker";
}
