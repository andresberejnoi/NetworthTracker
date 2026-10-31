-- Optional read-only aggregation configuration and provenance.
--
-- Provider data is stored separately from manual balances so a sync never
-- silently discards a user's value. Tokens are encrypted before storage.

CREATE TABLE IF NOT EXISTS aggregation_configs (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider          TEXT NOT NULL,
    enabled           BOOLEAN NOT NULL DEFAULT FALSE,
    token_ciphertext  TEXT,
    token_nonce       TEXT,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT aggregation_configs_provider_unique UNIQUE (provider)
);

CREATE TABLE IF NOT EXISTS aggregation_links (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider             TEXT NOT NULL,
    provider_account_id  TEXT NOT NULL,
    account_id           UUID NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    display_name         TEXT NOT NULL,
    account_type         TEXT NOT NULL,
    currency             TEXT NOT NULL,
    last_synced_at       TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT aggregation_links_unique UNIQUE (provider, provider_account_id),
    CONSTRAINT aggregation_links_currency CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE TABLE IF NOT EXISTS aggregation_balances (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    link_id     UUID NOT NULL REFERENCES aggregation_links (id) ON DELETE CASCADE,
    as_of       DATE NOT NULL,
    amount      NUMERIC NOT NULL,
    currency    TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL,
    CONSTRAINT aggregation_balances_unique UNIQUE (link_id, as_of),
    CONSTRAINT aggregation_balances_currency CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE TABLE IF NOT EXISTS aggregation_holdings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    link_id     UUID NOT NULL REFERENCES aggregation_links (id) ON DELETE CASCADE,
    symbol      TEXT NOT NULL,
    quantity    NUMERIC NOT NULL,
    value       NUMERIC NOT NULL,
    currency    TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL,
    CONSTRAINT aggregation_holdings_unique UNIQUE (link_id, symbol),
    CONSTRAINT aggregation_holdings_currency CHECK (currency ~ '^[A-Z]{3}$')
);
