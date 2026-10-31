-- Shared tax-lot ledger.
--
-- Lots are append-only facts. Sales consume lots and create disposal rows; the
-- original quantity and cost basis stay on the lot so history is reproducible.
-- Holding-period labels are deliberately not stored: they are derived from the
-- acquisition and disposal dates at query/report time.

CREATE TABLE IF NOT EXISTS tax_lots (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seq                   BIGSERIAL NOT NULL UNIQUE,
    holding_id            UUID NOT NULL REFERENCES holdings (id) ON DELETE CASCADE,
    acquired_on           DATE NOT NULL,
    original_quantity     NUMERIC NOT NULL,
    remaining_quantity    NUMERIC NOT NULL,
    original_cost_basis   NUMERIC NOT NULL,
    remaining_cost_basis  NUMERIC NOT NULL,
    currency              TEXT NOT NULL,
    fees                  NUMERIC NOT NULL DEFAULT 0,
    source                TEXT NOT NULL DEFAULT 'purchase',
    basis_quality         TEXT NOT NULL DEFAULT 'actual',
    basis_note            TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT tax_lots_original_quantity_positive CHECK (original_quantity > 0),
    CONSTRAINT tax_lots_remaining_quantity_range CHECK (
        remaining_quantity >= 0 AND remaining_quantity <= original_quantity
    ),
    CONSTRAINT tax_lots_original_basis_non_negative CHECK (original_cost_basis >= 0),
    CONSTRAINT tax_lots_remaining_basis_range CHECK (
        remaining_cost_basis >= 0 AND remaining_cost_basis <= original_cost_basis
    ),
    CONSTRAINT tax_lots_currency CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT tax_lots_fees_non_negative CHECK (fees >= 0),
    CONSTRAINT tax_lots_source CHECK (source IN (
        'purchase', 'income', 'transfer', 'import', 'adjustment'
    )),
    CONSTRAINT tax_lots_basis_quality CHECK (basis_quality IN ('actual', 'estimated'))
);

CREATE INDEX IF NOT EXISTS tax_lots_holding_acquired
    ON tax_lots (holding_id, acquired_on, seq);

-- Preserve existing lots. The previous table had no remaining/original split,
-- so both start at the recorded quantity and cost basis.
INSERT INTO tax_lots (
    holding_id, acquired_on, original_quantity, remaining_quantity,
    original_cost_basis, remaining_cost_basis, currency, fees, source,
    basis_quality, basis_note
)
SELECT
    holding_id, acquired_on, quantity, quantity,
    cost_basis, cost_basis, currency, 0, 'import',
    'actual', NULL
FROM holding_lots
WHERE quantity > 0;

DROP TABLE IF EXISTS holding_lots;

CREATE TABLE IF NOT EXISTS tax_sales (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    holding_id      UUID NOT NULL REFERENCES holdings (id) ON DELETE RESTRICT,
    disposed_on     DATE NOT NULL,
    quantity        NUMERIC NOT NULL,
    gross_proceeds  NUMERIC NOT NULL,
    fees            NUMERIC NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL,
    method          TEXT NOT NULL DEFAULT 'fifo',
    note            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT tax_sales_quantity_positive CHECK (quantity > 0),
    CONSTRAINT tax_sales_proceeds_non_negative CHECK (gross_proceeds >= 0),
    CONSTRAINT tax_sales_fees_non_negative CHECK (fees >= 0),
    CONSTRAINT tax_sales_currency CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT tax_sales_method CHECK (method IN ('fifo'))
);

CREATE INDEX IF NOT EXISTS tax_sales_holding_disposed
    ON tax_sales (holding_id, disposed_on, id);

CREATE TABLE IF NOT EXISTS tax_disposals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sale_id         UUID NOT NULL REFERENCES tax_sales (id) ON DELETE CASCADE,
    lot_id          UUID NOT NULL REFERENCES tax_lots (id) ON DELETE RESTRICT,
    quantity        NUMERIC NOT NULL,
    gross_proceeds  NUMERIC NOT NULL,
    fees            NUMERIC NOT NULL DEFAULT 0,
    cost_basis      NUMERIC NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT tax_disposals_quantity_positive CHECK (quantity > 0),
    CONSTRAINT tax_disposals_proceeds_non_negative CHECK (gross_proceeds >= 0),
    CONSTRAINT tax_disposals_fees_non_negative CHECK (fees >= 0),
    CONSTRAINT tax_disposals_basis_non_negative CHECK (cost_basis >= 0)
);

CREATE INDEX IF NOT EXISTS tax_disposals_sale
    ON tax_disposals (sale_id);

CREATE INDEX IF NOT EXISTS tax_disposals_lot
    ON tax_disposals (lot_id);
