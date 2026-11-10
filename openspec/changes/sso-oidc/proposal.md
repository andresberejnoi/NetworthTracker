# Proposal

## Why

The app currently authenticates only with local usernames and passwords. A
homelab already running a central identity provider should be able to use its
existing OpenID Connect (OIDC) SSO instead of maintaining a separate password
for this service, while keeping local login available as a break-glass path.

## What Changes

- Add optional OpenID Connect sign-in through a user-configured IdP, with
  discovery, client credentials, scopes, and issuer settings supplied by the
  user.
- Use the Authorization Code flow with PKCE, state, nonce, and full ID-token
  validation; do not trust email or display-name claims as identity keys.
- Add a stable external-identity mapping keyed by `issuer + subject`, linked to
  the existing local user and role model.
- Support explicit account linking for existing local users and optional
  first-login provisioning for new users.
- Add allowlist and group/claim role mapping so SSO users receive owner, editor,
  or viewer permissions without silently becoming owners.
- Keep local password login and the existing bearer-session model as a
  fallback; if the IdP is unavailable, the app must remain locally usable.
- Add an SSO button/status to the frontend, a callback exchange, and owner-only
  linking/audit visibility.
- Add mock-provider tests and documentation for Authentik, Keycloak, Pocket ID,
  Authelia, or another OIDC-compatible provider.

No **BREAKING** API changes are required. Existing local users, sessions, and
the `AUTH_DISABLED` single-user mode continue to work.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity`: authentication now supports optional OIDC SSO in addition to
  local credentials, with external identity linking, provisioning, role
  mapping, and local fallback.

## Impact

- `backend/crates/application/src/auth.rs` and a new OIDC client/service module.
- `IdentityRepository` and both memory/Postgres implementations.
- New migration for `auth_identities`, short-lived OIDC state, one-time
  exchange codes, and nullable local password hashes for SSO-only users.
- New API endpoints and OpenAPI schemas for auth status, start/callback/
  exchange, and account linking.
- Frontend login screen, SSO callback handling, API client, and tests.
- Configuration (`OIDC_*`), `docs/sso.md`, deployment/security docs, and
  mock-IdP test coverage.
- No new required external service for offline/local-password use.
