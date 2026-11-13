import { Link } from "react-router-dom";
import PageHeader from "../../components/PageHeader";
import BulkEntryView from "../bulk-entry/BulkEntryView";

export default function AccountsBulkPage() {
  return (
    <section>
      <PageHeader
        title="Bulk entry"
        description="Paste or type many dated balances for one account."
        actions={
          <Link className="button-link secondary" to="/accounts">
            Back to accounts
          </Link>
        }
      />
      <BulkEntryView />
    </section>
  );
}
