import { Link } from "react-router-dom";
import EmptyState from "../components/EmptyState";
import PageHeader from "../components/PageHeader";

export default function NotFound() {
  return (
    <section>
      <PageHeader title="Not found" />
      <EmptyState
        title="This page does not exist"
        description="The address may be outdated, or the view may have moved."
        action={<Link className="button-link" to="/">Go to Home</Link>}
      />
    </section>
  );
}
