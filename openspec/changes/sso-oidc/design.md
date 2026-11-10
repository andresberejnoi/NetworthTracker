# Design

## Decisions

### D1. Native OIDC, not only reverse-proxy auth

**Decision:** Implement OIDC in the backend using the Authorization Code flow
with PKCE. Keep local password login as a fallback.

**Rationale:** The app already owns users, roles, sessions, and per-user data
scoping. Native OIDC preserves those semantics and avoids trusting spoofable
proxy headers. A reverse-proxy forward-auth setup remains possible for
deployments that isolate the backend, but is not the primary integration.

**Alternatives considered:** Authelia/oauth2-proxy header trust (rejected as the
only path: backend must be unreachable except through the proxy, and role/user
provisioning becomes proxy-specific); embedding a full OAuth authorization
server (rejected: wrong direction for this app).

### D2. Authorization Code + PKCE with a confidential backend client

**Decision:** Use discovery from `OIDC_ISSUER/.well-known/openid-configuration`,
Authorization Code flow, PKCE S256, random state and nonce, and server-side code
exchange. The backend holds the client secret when configured.

**Rationale:** This is the standard interactive-login flow for a web app. The
backend can validate the ID token, keep the client secret off the browser, and
support providers that require a confidential client.

**Alternatives considered:** Implicit flow (rejected: deprecated/leaky);
password grant (rejected: not OIDC login and incompatible with MFA/passkeys).

### D3. External identity table keyed by issuer + subject

**Decision:** Add `auth_identities` with `(issuer, subject)` unique, linked to a
local `users` row. Email, username, and display name are metadata, never the
identity key.

**Rationale:** `sub` is the provider's stable identifier. Email can change and
`email_verified` semantics vary; auto-linking by email would allow account
takeover in a shared household. Existing local users link explicitly while
signed in.

### D4. SSO-only users have no local password

**Decision:** Make `users.password_hash` nullable. Local login rejects a user
without a usable hash; SSO users are created without one. Local-only users keep
their existing hashes.

**Rationale:** A random unusable hash is possible, but nullable records the
actual state and allows an owner to later attach a break-glass password.
Constraints and tests must ensure local login never succeeds with a null hash.

### D5. One-time exchange code, not a bearer token in the callback URL

**Decision:** The OIDC callback validates the login and creates a short-lived,
single-use exchange code bound to the user/session. It redirects to the frontend
with that code; the frontend exchanges it for the normal bearer session token.

**Rationale:** The SPA currently stores bearer sessions and does not use
cookies. Putting the long-lived session token in a URL leaks it through history,
logs, and referrers. A one-time code limits exposure and keeps the existing
client/session model.

**Alternatives considered:** HttpOnly cookie sessions (stronger, but a larger
frontend/API change; can be a later hardening step); token in URL fragment
(rejected: still leaks in some surfaces and is harder to audit).

### D6. Explicit provisioning, allowlist, and role mapping

**Decision:** OIDC login does not auto-create users unless
`OIDC_AUTO_PROVISION=true`. Allowlists can match verified email or subject.
Roles come from configured group/claim mappings; the default is viewer. Owner
escalation requires an explicit mapping. Existing local users are linked only
through an authenticated linking action.

**Rationale:** A homelab IdP may contain many users and groups. Automatic owner
creation is unacceptable, and silent email linking is unsafe.

### D7. Local login and `AUTH_DISABLED` remain independent

**Decision:** `OIDC_ENABLED` is independent of local auth. If OIDC is off or the
IdP is unavailable, local login still works. `AUTH_DISABLED=true` remains the
single-user offline mode and bypasses both login paths.

**Rationale:** A homelab outage must not lock the owner out of their financial
data, and offline-first remains a core product requirement.

### D8. Reuse sessions and roles; do not persist provider tokens

**Decision:** After successful OIDC login, issue the existing hashed-token
session (30 days) and use the existing role checks. Store only the external
identity link and audit metadata; do not store access, refresh, or ID tokens.

**Rationale:** The app does not call provider APIs after login, so retained
tokens add risk without value. The existing session model already supports
logout and role changes.

## Configuration

```env
OIDC_ENABLED=true
OIDC_ISSUER=https://auth.home.example/application/o/networth/
OIDC_CLIENT_ID=networth
OIDC_CLIENT_SECRET=...
OIDC_REDIRECT_URI=http://localhost:8080/api/v1/auth/oidc/callback
OIDC_FRONTEND_CALLBACK=http://localhost:3000/auth/oidc/callback
OIDC_SCOPES=openid profile email groups
OIDC_BUTTON_LABEL=Sign in with Homelab SSO
OIDC_AUTO_PROVISION=true
OIDC_DEFAULT_ROLE=viewer
OIDC_ALLOWED_EMAILS=you@example.com,family@example.com
OIDC_ALLOWED_SUBJECTS=...
OIDC_ROLE_CLAIM=groups
OIDC_OWNER_GROUPS=networth-owners
OIDC_EDITOR_GROUPS=networth-editors
```

Issuer discovery and endpoint URLs come from the discovery document; do not
hard-code provider-specific paths. For a local HTTP IdP, require an explicit
development-only opt-in rather than disabling TLS verification by default.

## Persistence

```sql
CREATE TABLE auth_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT,
    email_verified BOOLEAN,
    display_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ,
    UNIQUE (issuer, subject)
);

CREATE TABLE oidc_login_states (
    state TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    code_verifier TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE oidc_exchange_codes (
    code_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);

ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
```

State, verifier, and exchange codes expire quickly (for example 10 minutes and
60 seconds). Exchange codes are stored hashed and consumed atomically.

## API surface

```text
GET  /api/v1/auth/status             # include oidc_enabled + button label
GET  /api/v1/auth/oidc/start         # create state/nonce/PKCE, redirect
GET  /api/v1/auth/oidc/callback      # validate, link/provision, redirect with code
POST /api/v1/auth/oidc/exchange      # one-time code -> SessionDto
POST /api/v1/auth/oidc/link          # owner/authenticated-user linking
```

The callback redirect target should be configurable
(`OIDC_FRONTEND_CALLBACK`) and must not reflect arbitrary URLs.

## Validation and security

- Fetch/cache discovery and JWKS; handle key rotation by refreshing JWKS when a
  key id is unknown.
- Validate signature, `iss`, `aud`, `exp`, `iat`/`nbf` where present, and nonce.
- Require `email_verified` if email is used for an allowlist decision.
- Use constant-time/hashed comparisons for state and exchange codes.
- Rate-limit and audit failed attempts.
- Bound token/claim sizes and reject unexpected `alg` values; use the
  provider's advertised algorithms.
- Never log authorization codes, tokens, client secrets, or full claims.

## Provider notes

- Authentik/Keycloak/Pocket ID/Authelia: create an OIDC client with the exact
  callback URL, enable authorization code + PKCE, and request `openid profile
  email` (plus `groups` if role mapping is used).
- Ensure the provider's `groups` claim is emitted under the configured claim
  name; role mapping happens only after trusted ID-token validation.
- The IdP may live on the LAN. The backend must be able to reach the issuer and
  JWKS endpoints; browsers must reach the authorization endpoint.

## Testing strategy

- Mock OIDC discovery/token/JWKS server with valid and invalid fixtures.
- Cover valid login, bad state/nonce, expired/replayed code, wrong issuer/aud,
  bad signature, unknown user, allowlist, role groups, email collision, local
  fallback, provider outage, linking, logout, and token non-persistence.
- Add migration and repository tests for external identities and nullable
  password hashes.
- Add frontend tests for the SSO button/status and callback exchange.

## Risks / Trade-offs

- **IdP outage locks out SSO-only users** → keep local break-glass login and
  document an owner recovery path.
- **Force-moving provider groups can change roles unexpectedly** → define
  mapping semantics and audit role changes.
- **A local HTTP issuer exposes codes/tokens on the LAN** → prefer HTTPS; allow
  HTTP only with an explicit local-development setting.
- **Email linking is convenient but unsafe** → disabled by default and only
  possible with verified claims when explicitly enabled.
