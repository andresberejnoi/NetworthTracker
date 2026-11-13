import { useQuery } from "@tanstack/react-query";
import { API_BASE_URL, getPing } from "../../api/client";
import PageHeader from "../../components/PageHeader";

export default function AboutView() {
  const ping = useQuery({ queryKey: ["ping"], queryFn: ({ signal }) => getPing(signal) });

  return (
    <section>
      <PageHeader
        title="About"
        description="Version, API connection, and local-first operating notes."
      />
      <div className="card">
        <dl className="meta">
          <div>
            <dt>Service</dt>
            <dd>{ping.data?.service ?? "NetworthTracker"}</dd>
          </div>
          <div>
            <dt>Version</dt>
            <dd>{ping.data?.version ?? "—"}</dd>
          </div>
          <div>
            <dt>API</dt>
            <dd>{API_BASE_URL}</dd>
          </div>
          <div>
            <dt>Reply</dt>
            <dd>{ping.data?.message ?? (ping.isError ? "unreachable" : "checking…")}</dd>
          </div>
        </dl>
      </div>
      <div className="card stack">
        <h3>Local-first</h3>
        <p className="muted">
          Core account, net worth, import, and backup workflows run locally.
          External price, chain, aggregation, and SSO providers are optional and
          configured by the owner.
        </p>
      </div>
    </section>
  );
}
