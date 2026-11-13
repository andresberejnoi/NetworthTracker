import { Outlet } from "react-router-dom";
import { useCurrentUser } from "../auth/AuthContext";
import PageHeader from "../components/PageHeader";
import Tabs from "../components/Tabs";

export default function DataLayout() {
  const user = useCurrentUser();
  const items = [
    { to: "/data/import-export", label: "Import / export" },
    ...(user?.role === "owner"
      ? [
          { to: "/data/migration", label: "Migration" },
          { to: "/data/documents", label: "Documents" },
        ]
      : []),
  ];
  return (
    <section>
      <PageHeader
        title="Data"
        description="Move data in, keep original documents, and export what you own."
      />
      <Tabs label="Data sections" items={items} />
      <Outlet />
    </section>
  );
}
