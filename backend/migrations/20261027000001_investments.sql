-- Securities, holdings, tax lots, cached prices, and FX rates.

CREATE TABLE IF NOT EXISTS securities (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol      TEXT NOT NULL,
    name        TEXT NOT NULL,
    asset_class TEXT NOT NULL,
    sector      TEXT,
    currency    TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT securities_symbol_not_blank CHECK (length(btrim(symbol)) > 0),
    CONSTRAINT securities_asset_class CHECK (asset_class IN (
        'equity', 'fixed_income', 'real_estate', 'commodity', 'cash', 'other'
    )),
    CONSTRAINT securities_sector CHECK (sector IS NULL OR sector IN (
        'energy', 'materials', 'industrials', 'consumer_discretionary',
        'consumer_staples', 'health_care', 'financials',
        'information_technology', 'communication_services', 'utilities',
        'real_estate', 'other'
    )),
    CONSTRAINT securities_currency CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE UNIQUE INDEX IF NOT EXISTS securities_symbol_unique
    ON securities (upper(symbol));

CREATE TABLE IF NOT EXISTS holdings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    security_id UUID NOT NULL REFERENCES securities (id) ON DELETE RESTRICT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT holdings_one_per_security UNIQUE (account_id, security_id)
);

CREATE TABLE IF NOT EXISTS holding_lots (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    holding_id  UUID NOT NULL REFERENCES holdings (id) ON DELETE CASCADE,
    acquired_on DATE NOT NULL,
    quantity    NUMERIC NOT NULL,
    cost_basis  NUMERIC NOT NULL,
    currency    TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT holding_lots_quantity CHECK (quantity >= 0),
    CONSTRAINT holding_lots_currency CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE INDEX IF NOT EXISTS holding_lots_holding
    ON holding_lots (holding_id);

CREATE TABLE IF NOT EXISTS security_prices (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol     TEXT NOT NULL,
    price      NUMERIC NOT NULL,
    currency   TEXT NOT NULL,
    as_of      TIMESTAMPTZ NOT NULL,
    source     TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT security_prices_source CHECK (source IN ('provider', 'manual')),
    CONSTRAINT security_prices_currency CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE INDEX IF NOT EXISTS security_prices_symbol_asof
    ON security_prices (upper(symbol), as_of DESC);

CREATE TABLE IF NOT EXISTS fx_rates (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    base_currency  TEXT NOT NULL,
    quote_currency TEXT NOT NULL,
    rate           NUMERIC NOT NULL,
    as_of          TIMESTAMPTZ NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fx_rates_base CHECK (base_currency ~ '^[A-Z]{3}$'),
    CONSTRAINT fx_rates_quote CHECK (quote_currency ~ '^[A-Z]{3}$'),
    CONSTRAINT fx_rates_positive CHECK (rate > 0)
);

CREATE INDEX IF NOT EXISTS fx_rates_pair_asof
    ON fx_rates (base_currency, quote_currency, as_of DESC);
