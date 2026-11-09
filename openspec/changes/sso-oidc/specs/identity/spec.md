# Spec Delta

## Purpose

Extends the existing local identity model with optional OpenID Connect SSO for
homelab identity providers, without breaking offline/local-password access or
the existing role and session model.

## ADDED Requirements

### Requirement: Optional OIDC sign-in
The system SHALL support optional OpenID Connect sign-in when the owner
configures an issuer, client, redirect URI, and scopes, and SHALL keep the
feature disabled when those settings are absent.

#### Scenario: OIDC disabled
- **WHEN** OIDC settings are absent
- **THEN** the login screen shows the existing local form and no SSO button, and
  local authentication continues to work

#### Scenario: OIDC enabled
- **WHEN** a user selects "Sign in with SSO"
- **THEN** the browser is sent to the configured IdP authorization endpoint and
  a successful callback establishes a normal app session

### Requirement: Secure OIDC authorization flow
The system SHALL use the OIDC Authorization Code flow with PKCE, a random
`state`, a random `nonce`, exact redirect-URI validation, and ID-token
signature, issuer, audience, expiry, and nonce validation.

#### Scenario: Valid callback
- **WHEN** the IdP returns a valid authorization code and ID token for the
  configured issuer and client
- **THEN** the server validates the response and issues an app session

#### Scenario: Invalid callback
- **WHEN** `state` is missing/expired, `nonce` does not match, the audience or
  issuer is wrong, the signature is invalid, or the token is expired
- **THEN** the server rejects the login without creating a session or linking a
  user

#### Scenario: Replayed callback
- **WHEN** an authorization code, state, or one-time exchange code is reused
- **THEN** the second use is rejected

### Requirement: Stable external identity mapping
The system SHALL identify an SSO user by the pair `(issuer, subject)` and SHALL
NOT use email, username, or display name as the external identity key.

#### Scenario: Returning SSO user
- **WHEN** the same `issuer + subject` signs in again
- **THEN** the existing local user and role are reused without creating a
  duplicate profile

#### Scenario: Existing local account link
- **WHEN** an authenticated local user explicitly links an SSO identity
- **THEN** subsequent SSO logins with that identity resolve to the same local
  user

#### Scenario: Email collision
- **WHEN** an SSO email matches a local user but the subject is not linked and
  email auto-linking is disabled
- **THEN** the system does not silently take over the local account

### Requirement: Provisioning and role mapping
The system SHALL allow owners to configure allowlisted users and map IdP groups
or claims to the existing `owner`, `editor`, and `viewer` roles, with a
configurable default role and no implicit owner escalation.

#### Scenario: Allowlisted first login
- **WHEN** an allowlisted user signs in for the first time with provisioning
  enabled
- **THEN** a local profile is created with the configured role and linked to the
  external identity

#### Scenario: Unknown user
- **WHEN** SSO succeeds but the user is not linked, allowed, or
  provisionable
- **THEN** access is denied without creating a user or session

#### Scenario: Group role
- **WHEN** the configured role claim contains an owner/editor group
- **THEN** the resulting app role reflects the configured mapping and defaults
  to viewer when no mapping applies

### Requirement: Local fallback and break-glass access
The system SHALL keep local password authentication available while SSO is
enabled, and SHALL continue to allow the owner to use the existing
`AUTH_DISABLED` single-user mode independently.

#### Scenario: IdP unavailable
- **WHEN** the IdP discovery or token endpoint is unreachable during an SSO
  attempt
- **THEN** the error is reported and local password login remains available

#### Scenario: SSO-only user
- **WHEN** an SSO-provisioned user has no local password
- **THEN** password login rejects that user while SSO login continues to work

### Requirement: SSO session and secret handling
The system SHALL issue the existing hashed-token app session after SSO login,
SHALL NOT persist IdP access tokens, refresh tokens, or ID tokens, and SHALL
support local logout.

#### Scenario: Session established
- **WHEN** OIDC login succeeds
- **THEN** the user receives an app session with the same expiry and role
  behavior as a local login

#### Scenario: No provider tokens stored
- **WHEN** an SSO login completes
- **THEN** the database contains the external identity mapping and audit data,
  but not the IdP access/refresh/ID tokens

#### Scenario: Logout
- **WHEN** the user logs out
- **THEN** the local session token is invalidated; provider-side logout is
  optional

### Requirement: SSO audit and error reporting
The system SHALL record SSO login success, denial, linking, and configuration
failures with non-secret metadata, and SHALL return clear errors for
unconfigured, denied, or invalid SSO attempts.

#### Scenario: Audit event
- **WHEN** an SSO login succeeds or is denied
- **THEN** an audit event records the outcome, issuer, subject/identity
  reference, and timestamp without recording tokens

#### Scenario: Configuration failure
- **WHEN** the issuer or discovery document cannot be loaded while OIDC is
  enabled
- **THEN** the SSO endpoint reports a configuration/provider error and the local
  login path remains usable
