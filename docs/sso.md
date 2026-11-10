# SSO with OpenID Connect

> **Status:** this documents the integration contract planned in the
> `sso-oidc` change. It is not active in a released build yet. Local
> username/password login and `AUTH_DISABLED` remain the supported behavior
> until that change is implemented.

NetworthTracker uses **OpenID Connect (OIDC)** for single sign-on. OIDC is the
identity layer on top of OAuth 2.0: the identity provider (IdP) authenticates
the user, and the app receives a signed ID token containing a stable subject
(`sub`) and optional profile/group claims.

The app is the OIDC **client**. Your existing SSO service is the **provider**.

## Supported providers

Any standards-compliant OIDC provider with discovery, authorization code flow,
and PKCE should work. Typical homelab choices:

| Provider | Notes |
|---|---|
| **Authentik** | Create an OAuth2/OpenID provider and application; use its issuer URL and client secret. |
| **Keycloak** | Create a confidential client with Standard Flow + PKCE; add a group membership mapper if using groups. |
| **Pocket ID** | Create an OIDC client; use the discovery issuer and callback URL. |
| **Authelia** | Configure an OpenID Connect client in Authelia; ensure the issuer and callback match exactly. |
| **Auth0 / Okta / Entra ID / Google Workspace** | Create a regular web/OIDC application, enable code flow with PKCE where available, and add the callback URL. Group claims may require provider-specific configuration. |

Provider UI labels differ by version. Look for the equivalent of:

- Application type: **Web / regular web / confidential client**
- Grant type: **Authorization Code**
- PKCE: **S256 enabled/recommended**
- Redirect URI: the app callback below
- Scopes: `openid profile email` plus a groups/roles claim if needed

## App configuration

Add these to `.env` when the SSO feature is available:

```env
OIDC_ENABLED=true
OIDC_ISSUER=https://auth.example.com/application/o/networth/
OIDC_CLIENT_ID=networth-tracker
OIDC_CLIENT_SECRET=replace-with-a-long-random-secret
OIDC_REDIRECT_URI=https://networth.example.com/api/v1/auth/oidc/callback
OIDC_FRONTEND_CALLBACK=https://networth.example.com/auth/oidc/callback
OIDC_SCOPES=openid profile email groups
OIDC_BUTTON_LABEL=Sign in with Homelab SSO

# Provisioning and authorization policy
OIDC_AUTO_PROVISION=true
OIDC_DEFAULT_ROLE=viewer
OIDC_ALLOWED_EMAILS=you@example.com,partner@example.com
# OIDC_ALLOWED_SUBJECTS=provider-subject-1,provider-subject-2

# Optional group/claim role mapping
OIDC_ROLE_CLAIM=groups
OIDC_OWNER_GROUPS=networth-owners
OIDC_EDITOR_GROUPS=networth-editors
```

Notes:

- `OIDC_ISSUER` must be the issuer in the provider's discovery document, not a
  guessed URL. The app fetches
  `${OIDC_ISSUER}/.well-known/openid-configuration`.
- `OIDC_REDIRECT_URI` must match the provider registration **exactly**,
  including scheme, host, port, and trailing slash.
- `OIDC_FRONTEND_CALLBACK` is the SPA route that receives the one-time exchange
  code; it must be an allowed local URL, never an open redirect.
- `OIDC_CLIENT_SECRET` is only needed for a confidential client. If your
  provider requires a public client, leave it unset and keep PKCE enabled.
- `OIDC_SCOPES` must include `openid`; `profile` and `email` provide display
  data. Add `groups` or the provider's equivalent if mapping roles.
- Keep the client secret and `DOCUMENT_ENCRYPTION_KEY` out of backups and
  version control.

## Provider setup walkthroughs

### Authentik

1. Create an **OAuth2/OpenID Provider**.
2. Set the client type to **Confidential**.
3. Add the redirect URI:
   `https://networth.example.com/api/v1/auth/oidc/callback`
4. Enable the authorization code flow and PKCE.
5. Add scopes `openid`, `profile`, `email`, and a groups scope/mapping if you
   want role mapping.
6. Create an **Application** bound to the provider.
7. Use the provider's issuer URL for `OIDC_ISSUER` and its client ID/secret.
8. If using groups, configure the provider to emit the group claim under
   `groups` (or set `OIDC_ROLE_CLAIM` to the emitted claim name).

### Keycloak

1. Create a client in the target realm.
2. Set **Client authentication** on for a confidential client.
3. Enable **Standard flow** and **PKCE S256**.
4. Set **Valid redirect URIs** to the app callback.
5. Add the web origin if required by the provider.
6. Use the realm issuer, for example
   `https://keycloak.example.com/realms/homelab`.
7. Add a **Group Membership** mapper to the ID token or userinfo under the
   claim name used by `OIDC_ROLE_CLAIM`.
8. Ensure the client scopes include `openid`, `profile`, and `email`.

### Pocket ID

1. Create an OIDC client.
2. Add the exact callback URL.
3. Use the Pocket ID issuer and client credentials.
4. Enable PKCE; use a confidential client if supported, otherwise leave the
   secret unset.
5. Add groups/roles to the token if you want automatic role mapping.

### Authelia

1. Configure an OpenID Connect client in Authelia.
2. Register the callback URL exactly.
3. Enable the authorization code flow and PKCE.
4. Use the Authelia issuer URL.
5. If group-based roles are required, verify Authelia's emitted claims and set
   `OIDC_ROLE_CLAIM` accordingly.

### Hosted providers

For Auth0, Okta, Entra ID, Google Workspace, or another hosted provider:

1. Create a web application / OIDC client.
2. Add the callback URL.
3. Enable authorization code flow with PKCE where available.
4. Request `openid profile email`; add a groups/roles claim if your plan and
   configuration support it.
5. Copy the issuer, client ID, and client secret into `.env`.
6. If the provider cannot emit groups, leave role mapping unset and manage roles
   in NetworthTracker's Users view after linking/provisioning.

## Login flow

1. The login page shows **Sign in with Homelab SSO** when OIDC is enabled.
2. The browser is redirected to the provider with `state`, `nonce`, and a PKCE
   challenge.
3. After authentication, the provider redirects to
   `/api/v1/auth/oidc/callback`.
4. The backend validates the response and ID token, resolves the external
   identity by `(issuer, subject)`, and links or provisions the local user.
5. The backend redirects to the frontend with a short-lived one-time code.
6. The frontend exchanges that code for the normal app session token.

The app does not retain the provider's access token, refresh token, or ID token
after login. Only the external identity link and audit metadata are stored.

## Roles and provisioning

Recommended policy:

| Situation | Result |
|---|---|
| Subject already linked | Reuse the local user and current role |
| Allowlisted new user, auto-provision on | Create user with `OIDC_DEFAULT_ROLE` |
| Not linked and not allowlisted | Deny login |
| Owner group in role claim | `owner` |
| Editor group in role claim | `editor` |
| No mapped group | `OIDC_DEFAULT_ROLE` (default `viewer`) |

Do not use email as the identity key. Email can change, and silently linking an
SSO email to an existing local account can let another IdP user take over that
account. Existing local users should link explicitly from an authenticated
session. Email auto-linking should remain opt-in and should require a verified
email claim.

## Local fallback and break-glass access

Keep local login available:

- If the IdP is down, an owner can still sign in with a local password.
- An SSO-provisioned user has no local password; create or attach a local
  password for the owner account before relying on SSO as the only login.
- `AUTH_DISABLED=true` continues to mean single-user offline mode and does not
  require the IdP.
- Do not remove the original owner credentials until SSO login and role mapping
  have been tested.

## Reverse-proxy alternative

If you already run Authelia, oauth2-proxy, or a similar forward-auth layer, you
can put NetworthTracker behind it instead of using native OIDC. This is a
different deployment model:

- The backend must be reachable **only** through the proxy, never directly from
  the LAN.
- The app must trust an authenticated header such as `Remote-User` and
  `Remote-Groups` only from that proxy.
- Roles and user provisioning become proxy/header configuration, not app OIDC
  configuration.
- Do not enable header trust on a directly reachable backend; headers are
  spoofable otherwise.

Native OIDC is preferable when you want the app's existing user/role model and
local fallback to stay authoritative.

## Troubleshooting

| Symptom | Check |
|---|---|
| No SSO button | OIDC is disabled, discovery failed, or the issuer/client config is incomplete |
| `redirect_uri_mismatch` | Provider callback differs from `OIDC_REDIRECT_URI` by scheme, host, port, path, or slash |
| Issuer mismatch | `OIDC_ISSUER` does not exactly match the `iss` in the discovery document/ID token |
| Login succeeds at IdP but app denies | User is not linked/provisioned or is outside `OIDC_ALLOWED_EMAILS`/subjects |
| Role is always viewer | Group claim is absent, under a different name, or the mapping is not configured |
| Email is present but account is not linked | Email auto-linking is intentionally disabled; link explicitly |
| Local login fails after SSO setup | SSO-only users intentionally have no local password; keep a local owner credential |
| IdP is on a self-signed HTTPS certificate | Trust the local CA on the backend; do not disable TLS verification globally |
| Provider unreachable | SSO fails, but local login remains available; check DNS, firewall, issuer URL, and JWKS access |

## Security checklist

- Use HTTPS for the app and IdP outside localhost.
- Keep the client secret in `.env`, mode `600`, not in Git or backups.
- Enable PKCE and authorization code flow.
- Keep state/nonce validation enabled; never accept a callback without them.
- Validate issuer, audience, signature, expiry, and nonce.
- Require verified email before using email in an allowlist decision.
- Do not log authorization codes, tokens, or full claims.
- Audit SSO success/denial and role changes.
- Test with a non-owner account before making SSO mandatory.

## Documentation status

This guide is the planned operational contract for the `sso-oidc` OpenSpec
change. When the change is implemented, the configuration names, callback
routes, and behavior become the supported interface and should be verified
against a locally running Authentik, Keycloak, Pocket ID, Authelia, or hosted
provider.
