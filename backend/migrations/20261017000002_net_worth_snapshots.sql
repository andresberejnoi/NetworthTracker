-- Immutable net worth snapshots. Each row records the totals, the inputs, and
-- the redemption policy used, so history stays reproducible when assumptions
-- change later.

CREATE TABLE IF NOT EXISTS net_worth_snapshots (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    as_of          DATE NOT NULL,
    scope          TEXT NOT NULL,
    owner_user_id  UUID,
    owner_age      INTEGER,
    currency       TEXT NOT NULL,
    total          NUMERIC NOT NULL,
    liquid         NUMERIC NOT NULL,
    accessible     NUMERIC NOT NULL,
    tax_rate       NUMERIC NOT NULL,
    penalty_rate   NUMERIC NOT NULL,
    penalty_age    INTEGER NOT NULL,
    selling_rate   NUMERIC NOT NULL,
    fixed_costs    NUMERIC NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT net_worth_snapshots_scope CHECK (scope IN ('household', 'personal')),
    CONSTRAINT net_worth_snapshots_currency CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE INDEX IF NOT EXISTS idx_net_worth_snapshots_as_of
    ON net_worth_snapshots (as_of);
