-- Encrypted document store metadata, audit trail, and import provenance.
--
-- File bytes live in the encrypted content-addressed store on disk; this schema
-- records immutable content hashes, classification, tags, and audit events.

CREATE TABLE IF NOT EXISTS documents (
    id                        UUID PRIMARY KEY,
    sha256                    TEXT NOT NULL UNIQUE,
    original_name             TEXT NOT NULL,
    mime_type                 TEXT NOT NULL,
    byte_size                 BIGINT NOT NULL,
    uploader_user_id          UUID,
    tags                      TEXT[] NOT NULL DEFAULT '{}',
    institution               TEXT,
    document_type             TEXT,
    classification_confidence DOUBLE PRECISION,
    parser_version            TEXT,
    status                    TEXT NOT NULL DEFAULT 'uploaded',
    period                    TEXT,
    parse_error               TEXT,
    nonce                     TEXT NOT NULL,
    key_version               TEXT NOT NULL DEFAULT '1',
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at                TIMESTAMPTZ,
    CONSTRAINT documents_size_non_negative CHECK (byte_size >= 0),
    CONSTRAINT documents_sha256_shape CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT documents_confidence_range CHECK (
        classification_confidence IS NULL
        OR (classification_confidence >= 0 AND classification_confidence <= 1)
    )
);

CREATE INDEX IF NOT EXISTS documents_created
    ON documents (created_at DESC);

CREATE TABLE IF NOT EXISTS document_audit_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents (id) ON DELETE CASCADE,
    user_id     UUID,
    action      TEXT NOT NULL,
    metadata    JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS document_audit_document
    ON document_audit_events (document_id, created_at);

CREATE TABLE IF NOT EXISTS document_provenance (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id  UUID NOT NULL REFERENCES documents (id) ON DELETE CASCADE,
    migration_id UUID NOT NULL REFERENCES migration_runs (id) ON DELETE CASCADE,
    entity_type  TEXT NOT NULL,
    entity_id    UUID NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS document_provenance_document
    ON document_provenance (document_id, entity_type, entity_id);
