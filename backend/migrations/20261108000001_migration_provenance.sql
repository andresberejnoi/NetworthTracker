-- Guided migration runs and provenance links.
--
-- A migration run groups the records written by one previewed/confirmed file.
-- Provenance rows point at the created or updated entity so imported data can
-- be traced to its source without storing the source file in the database.

CREATE TABLE IF NOT EXISTS migration_runs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source      TEXT NOT NULL,
    summary     JSONB NOT NULL DEFAULT '{}'::jsonb,
    imported_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS migration_provenance (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    migration_id UUID NOT NULL REFERENCES migration_runs (id) ON DELETE CASCADE,
    entity_type  TEXT NOT NULL,
    entity_id    UUID NOT NULL,
    source       TEXT NOT NULL,
    imported_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (migration_id, entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS migration_provenance_entity
    ON migration_provenance (entity_type, entity_id);
