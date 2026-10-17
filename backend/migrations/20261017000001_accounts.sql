-- Accounts and their dated balances.
--
-- Monetary amounts use NUMERIC so they are exact; they are never REAL.

CREATE TABLE IF NOT EXISTS accounts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name             TEXT NOT NULL,
    account_type     TEXT NOT NULL,
    institution      TEXT,
    currency         TEXT NOT NULL,
    ownership_scope  TEXT NOT NULL DEFAULT 'household',
    owner_user_id    UUID,
    archived         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT accounts_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT accounts_type_known CHECK (account_type IN (
        '401k', 'traditional_ira', 'roth_ira', 'taxable_brokerage', 'checking',
        'savings', 'cash', 'real_estate', 'other_asset', 'mortgage', 'loan'
    )),
    CONSTRAINT accounts_currency_code CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT accounts_ownership_scope CHECK (ownership_scope IN ('household', 'user'))
);

CREATE TABLE IF NOT EXISTS account_balances (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts (id) ON DELETE RESTRICT,
    as_of       DATE NOT NULL,
    -- Unbounded NUMERIC (not NUMERIC(p,s)) so the entered scale is preserved
    -- exactly: fiat keeps its cents and crypto keeps up to 18+ decimals.
    amount      NUMERIC NOT NULL,
    currency    TEXT NOT NULL,
    source      TEXT NOT NULL DEFAULT 'manual',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT account_balances_currency_code CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT account_balances_source CHECK (source IN ('manual', 'import', 'provider')),
    CONSTRAINT account_balances_one_per_day UNIQUE (account_id, as_of)
);

CREATE INDEX IF NOT EXISTS idx_account_balances_account_date
    ON account_balances (account_id, as_of DESC);
