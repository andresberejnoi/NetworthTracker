import { useMutation, useQuery } from "@tanstack/react-query";
import { getAuthStatus, startOidcLink } from "../../api/client";
import PageHeader from "../../components/PageHeader";

export default function SsoSettings() {
  const status = useQuery({ queryKey: ["auth-status"], queryFn: getAuthStatus });
  const link = useMutation({
    mutationFn: startOidcLink,
    onSuccess: (result) => {
      window.location.href = result.authorization_url;
    },
  });

  return (
    <section>
      <PageHeader
        title="SSO"
        description="Connect this household to an existing OpenID Connect identity provider."
      />
      <div className="card">
        <div className="status-row">
          <div>
            <h3>OpenID Connect</h3>
            <p className="muted">
              {status.data?.oidc_enabled
                ? `Enabled · ${status.data.oidc_button_label ?? "Sign in with SSO"}`
                : "Not configured"}
            </p>
          </div>
          <span className={status.data?.oidc_enabled ? "badge" : "badge badge-warn"}>
            {status.data?.oidc_enabled ? "configured" : "local login only"}
          </span>
        </div>

        {status.data?.oidc_enabled ? (
          <div className="bulk-actions">
            <button
              type="button"
              onClick={() => link.mutate()}
              disabled={link.isPending}
            >
              {link.isPending ? "Opening provider…" : "Link my SSO identity"}
            </button>
            {link.isError && (
              <span className="error" role="alert">
                {(link.error as Error).message}
              </span>
            )}
          </div>
        ) : (
          <p className="muted">
            Set the `OIDC_*` variables in `.env`, restart the backend, then sign
            in with the provider or link an existing local account.
          </p>
        )}

        <h3>Configuration</h3>
        <p className="muted">
          Issuer, client ID/secret, callback URL, scopes, allowlist, and
          group-to-role mapping are configured through environment variables.
          Client secrets are never displayed in the app.
        </p>
        <p>
          Read the full provider setup and troubleshooting guide in{" "}
          <code>docs/sso.md</code> in the repository.
        </p>
      </div>
    </section>
  );
}
