-- Cryptocurrency assets, wallets, and dated balances.
--
-- A wallet is a pool: either a manual bucket (exchange, hardware wallet,
-- "total") or a public address. Balances are dated facts; address sync writes
-- source = 'chain', manual entry writes source = 'manual'. Tax lots remain in
-- the shared tax_lots ledger and can reference a crypto asset + wallet.

CREATE TABLE IF NOT EXISTS crypto_assets (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol             TEXT NOT NULL,
    name               TEXT NOT NULL,
    chain              TEXT NOT NULL,
    contract_address   TEXT,
    decimals           INTEGER NOT NULL DEFAULT 18,
    currency           TEXT NOT NULL DEFAULT 'USD',
    price_provider_id  TEXT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crypto_assets_symbol_not_blank CHECK (length(btrim(symbol)) > 0),
    CONSTRAINT crypto_assets_chain_known CHECK (chain IN ('bitcoin', 'ethereum')),
    CONSTRAINT crypto_assets_decimals CHECK (decimals BETWEEN 0 AND 36),
    CONSTRAINT crypto_assets_currency CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT crypto_assets_unique_symbol_chain UNIQUE (chain, symbol)
);

CREATE TABLE IF NOT EXISTS crypto_wallets (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id         UUID NOT NULL REFERENCES crypto_assets (id) ON DELETE RESTRICT,
    kind             TEXT NOT NULL,
    chain            TEXT NOT NULL,
    address          TEXT,
    label            TEXT NOT NULL,
    ownership_scope  TEXT NOT NULL DEFAULT 'household',
    owner_user_id    UUID REFERENCES users (id) ON DELETE SET NULL,
    archived         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crypto_wallets_kind CHECK (kind IN ('address', 'manual')),
    CONSTRAINT crypto_wallets_chain_known CHECK (chain IN ('bitcoin', 'ethereum')),
    CONSTRAINT crypto_wallets_label_not_blank CHECK (length(btrim(label)) > 0),
    CONSTRAINT crypto_wallets_ownership_scope CHECK (
        ownership_scope IN ('household', 'user')
    ),
    CONSTRAINT crypto_wallets_address_required CHECK (
        kind = 'manual' OR address IS NOT NULL
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS crypto_wallets_address_unique
    ON crypto_wallets (asset_id, chain, address)
    WHERE address IS NOT NULL;

CREATE INDEX IF NOT EXISTS crypto_wallets_asset
    ON crypto_wallets (asset_id);

CREATE TABLE IF NOT EXISTS crypto_balances (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id   UUID NOT NULL REFERENCES crypto_wallets (id) ON DELETE CASCADE,
    as_of       DATE NOT NULL,
    quantity    NUMERIC NOT NULL,
    source      TEXT NOT NULL,
    provider    TEXT,
    fetched_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crypto_balances_quantity_non_negative CHECK (quantity >= 0),
    CONSTRAINT crypto_balances_source CHECK (source IN ('manual', 'chain', 'import')),
    CONSTRAINT crypto_balances_one_per_source_day UNIQUE (wallet_id, as_of, source)
);

CREATE INDEX IF NOT EXISTS crypto_balances_wallet_date
    ON crypto_balances (wallet_id, as_of DESC);

-- Accounts now understand a synthetic crypto account type (used for net worth
-- aggregation; crypto wallets remain the source records).
ALTER TABLE accounts DROP CONSTRAINT IF EXISTS accounts_type_known;
ALTER TABLE accounts ADD CONSTRAINT accounts_type_known CHECK (account_type IN (
    '401k', 'traditional_ira', 'roth_ira', 'taxable_brokerage', 'checking',
    'savings', 'cash', 'real_estate', 'other_asset', 'mortgage', 'loan', 'crypto'
));

-- The shared tax-lot ledger can now be owned by a crypto wallet as well as a
-- securities holding. Existing rows remain valid.
ALTER TABLE tax_lots ALTER COLUMN holding_id DROP NOT NULL;
ALTER TABLE tax_lots ADD COLUMN IF NOT EXISTS crypto_asset_id UUID
    REFERENCES crypto_assets (id) ON DELETE RESTRICT;
ALTER TABLE tax_lots ADD COLUMN IF NOT EXISTS wallet_id UUID
    REFERENCES crypto_wallets (id) ON DELETE CASCADE;

ALTER TABLE tax_lots ADD CONSTRAINT tax_lots_owner_exactly_one CHECK (
    (holding_id IS NOT NULL)::int + (wallet_id IS NOT NULL)::int = 1
);
ALTER TABLE tax_lots ADD CONSTRAINT tax_lots_asset_matches_owner CHECK (
    (holding_id IS NOT NULL AND crypto_asset_id IS NULL)
    OR (wallet_id IS NOT NULL AND crypto_asset_id IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS tax_lots_crypto_asset
    ON tax_lots (crypto_asset_id, acquired_on, seq)
    WHERE crypto_asset_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS tax_lots_wallet
    ON tax_lots (wallet_id, acquired_on, seq)
    WHERE wallet_id IS NOT NULL;

ALTER TABLE tax_sales ALTER COLUMN holding_id DROP NOT NULL;
ALTER TABLE tax_sales ADD COLUMN IF NOT EXISTS crypto_asset_id UUID
    REFERENCES crypto_assets (id) ON DELETE RESTRICT;
ALTER TABLE tax_sales ADD COLUMN IF NOT EXISTS wallet_id UUID
    REFERENCES crypto_wallets (id) ON DELETE CASCADE;

ALTER TABLE tax_sales ADD CONSTRAINT tax_sales_owner_exactly_one CHECK (
    (holding_id IS NOT NULL)::int + (wallet_id IS NOT NULL)::int = 1
);
ALTER TABLE tax_sales ADD CONSTRAINT tax_sales_asset_matches_owner CHECK (
    (holding_id IS NOT NULL AND crypto_asset_id IS NULL)
    OR (wallet_id IS NOT NULL AND crypto_asset_id IS NOT NULL)
);
