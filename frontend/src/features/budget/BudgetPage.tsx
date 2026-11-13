import PageHeader from "../../components/PageHeader";
import BudgetView from "./BudgetView";

export default function BudgetPage() {
  return (
    <section>
      <PageHeader
        title="Budget"
        description="Optional annual income, spending plan, and plan-versus-actual tracking."
      />
      <BudgetView />
    </section>
  );
}
