import { Outlet } from "react-router-dom";
import PageHeader from "../components/PageHeader";
import Tabs from "../components/Tabs";
import { settingsNavigation } from "./navigation";

export default function SettingsLayout() {
  return (
    <section>
      <PageHeader
        title="Settings"
        description="Household access, integrations, backups, and diagnostics."
      />
      <Tabs
        label="Settings sections"
        items={settingsNavigation.map((item) => ({
          to: item.to,
          label: item.label,
        }))}
      />
      <Outlet />
    </section>
  );
}
