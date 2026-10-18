-- Snapshots now record liquidity (cash on hand) in addition to total, liquid,
-- and accessible net worth. Existing rows default to zero; snapshots are
-- immutable, so they are intentionally not recomputed.
ALTER TABLE net_worth_snapshots
    ADD COLUMN IF NOT EXISTS liquidity NUMERIC NOT NULL DEFAULT 0;
