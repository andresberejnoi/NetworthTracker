-- Optional OpenID Connect identity links and short-lived login state.

ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

CREATE TABLE IF NOT EXISTS auth_identities (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    issuer         TEXT NOT NULL,
    subject        TEXT NOT NULL,
    email          TEXT,
    email_verified BOOLEAN NOT NULL DEFAULT false,
    display_name   TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at  TIMESTAMPTZ,
    UNIQUE (issuer, subject)
);

CREATE INDEX IF NOT EXISTS auth_identities_user
    ON auth_identities (user_id);

CREATE TABLE IF NOT EXISTS oidc_login_states (
    state         TEXT PRIMARY KEY,
    nonce         TEXT NOT NULL,
    code_verifier TEXT NOT NULL,
    link_user_id  UUID REFERENCES users (id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS oidc_login_states_expiry
    ON oidc_login_states (expires_at);

CREATE TABLE IF NOT EXISTS oidc_exchange_codes (
    code_hash  TEXT PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS oidc_exchange_codes_expiry
    ON oidc_exchange_codes (expires_at);

CREATE TABLE IF NOT EXISTS auth_audit_events (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID REFERENCES users (id) ON DELETE SET NULL,
    action     TEXT NOT NULL,
    issuer     TEXT NOT NULL,
    subject    TEXT NOT NULL,
    metadata   JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS auth_audit_events_created
    ON auth_audit_events (created_at DESC);
