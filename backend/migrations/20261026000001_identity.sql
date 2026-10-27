-- Households, users, and sessions.
--
-- v1 has a single implicit household; the row exists so users have a stable
-- parent and multi-household support can be added later without a rewrite.

CREATE TABLE IF NOT EXISTS households (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO households (id, name)
VALUES ('00000000-0000-0000-0000-000000000001', 'Household')
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    household_id  UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001'
                  REFERENCES households (id) ON DELETE CASCADE,
    username      TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT users_role CHECK (role IN ('owner', 'editor', 'viewer'))
);

CREATE UNIQUE INDEX IF NOT EXISTS users_username_unique
    ON users (lower(username));

CREATE TABLE IF NOT EXISTS sessions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS sessions_user
    ON sessions (user_id);
