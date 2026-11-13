import { Outlet } from "react-router-dom";
import PageHeader from "../components/PageHeader";
import Tabs from "../components/Tabs";

export default function PortfolioLayout() {
  return (
    <section>
      <PageHeader
        title="Portfolio"
        description="Securities, tax lots, crypto wallets, and positions."
      />
      <Tabs
        label="Portfolio sections"
        items={[
          { to: "/portfolio/investments", label: "Investments" },
          { to: "/portfolio/crypto", label: "Crypto" },
        ]}
      />
      <Outlet />
    </section>
  );
}
