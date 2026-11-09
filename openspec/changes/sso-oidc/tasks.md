# Tasks

## 1. Configuration and status

- [ ] 1.1 Add `OIDC_ENABLED`, issuer, client, redirect, scopes, button label, provisioning, allowlist, and role-mapping configuration
- [ ] 1.2 Make OIDC optional and fail-safe: absent/invalid config disables SSO without affecting local login or `AUTH_DISABLED`
- [ ] 1.3 Extend auth status with `oidc_enabled` and the configured SSO button label
- [ ] 1.4 Document the new environment variables in `.env.example` and deployment docs

## 2. Persistence and identity model

- [ ] 2.1 Add migrations for `auth_identities`, `oidc_login_states`, one-time `oidc_exchange_codes`, and nullable local password hashes
- [ ] 2.2 Add repository methods for finding/creating external identities, updating last login, storing/consuming OIDC state, and exchanging one-time codes
- [ ] 2.3 Implement the new persistence methods in the memory and PostgreSQL adapters with tests
- [ ] 2.4 Ensure local login rejects SSO-only users with no usable password hash

## 3. OIDC client and flow

- [ ] 3.1 Add the OIDC client dependency and discovery/JWKS loading with sane timeouts and caching
- [ ] 3.2 Implement the start endpoint: random state/nonce, PKCE S256, state persistence, and authorization redirect
- [ ] 3.3 Implement the callback: state/expiry validation, code exchange, ID-token signature/issuer/audience/expiry/nonce validation
- [ ] 3.4 Implement one-time exchange codes and atomic consumption for the SPA callback
- [ ] 3.5 Handle provider outage, malformed discovery, unknown key ids, and invalid responses without creating sessions
- [ ] 3.6 Define logout behavior and optional provider-side logout metadata

## 4. Identity linking, provisioning, and roles

- [ ] 4.1 Resolve returning users by `(issuer, subject)` and update last-login metadata
- [ ] 4.2 Implement authenticated account linking for existing local users
- [ ] 4.3 Implement allowlisted first-login provisioning with `OIDC_AUTO_PROVISION`
- [ ] 4.4 Implement group/claim-to-role mapping with default viewer and no implicit owner escalation
- [ ] 4.5 Prevent silent email-based takeover when a subject is not linked
- [ ] 4.6 Keep local password fallback and break-glass recovery available
- [ ] 4.7 Add non-secret audit events for SSO success, denial, linking, role mapping, and configuration failures

## 5. API and contract

- [ ] 5.1 Add auth status fields, start/callback/exchange/link routes, and DTOs
- [ ] 5.2 Map OIDC errors to clear API responses without leaking secrets or tokens
- [ ] 5.3 Update `contracts/openapi.yaml` and regenerate the frontend typed client
- [ ] 5.4 Add API tests for enabled/disabled SSO, successful login, denial, invalid callbacks, and linking

## 6. Frontend

- [ ] 6.1 Show the SSO button only when OIDC is enabled and configured
- [ ] 6.2 Add callback handling and one-time code exchange into the existing session flow
- [ ] 6.3 Add owner-only SSO linking/status visibility without exposing provider secrets
- [ ] 6.4 Add frontend tests for SSO visibility, callback success, and callback failure
- [ ] 6.5 Verify the full browser flow against a local mock provider

## 7. Security and verification

- [ ] 7.1 Add mock OIDC discovery/token/JWKS fixtures covering valid and invalid flows
- [ ] 7.2 Test state/nonce replay, expired codes, wrong issuer/audience, bad signatures, and unknown/denied users
- [ ] 7.3 Verify provider tokens are never persisted and secrets are never logged
- [ ] 7.4 Verify local login and `AUTH_DISABLED` still work when OIDC is absent, misconfigured, or unreachable
- [ ] 7.5 Add database and cross-boundary/integration coverage for SSO login and local fallback

## 8. Documentation and release

- [ ] 8.1 Add `docs/sso.md` with provider setup (Authentik/Keycloak/Pocket ID/Authelia), role mapping, linking, and break-glass recovery
- [ ] 8.2 Update deployment, security, README, CHANGELOG, and troubleshooting docs
- [ ] 8.3 Run the full release matrix and tag the next version
