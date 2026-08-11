-- Source-authoritative lifecycle semantics for provider-neutral evidence.
ALTER TABLE source_evidence_records
    ALTER COLUMN normalized_evidence DROP NOT NULL,
    ADD COLUMN is_current BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN is_tombstone BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN withdrawn_at TIMESTAMPTZ,
    ADD COLUMN superseded_at TIMESTAMPTZ,
    ADD COLUMN invalidation_reason VARCHAR(32),
    DROP CONSTRAINT source_evidence_trust_check,
    ADD CONSTRAINT source_evidence_trust_check CHECK (
        evidence_trust IN ('contract_trusted', 'model_derived_unreviewed')
    ),
    ADD CONSTRAINT source_evidence_lifecycle_shape_check CHECK (
        (is_tombstone AND normalized_evidence IS NULL AND withdrawn_at IS NOT NULL)
        OR
        (NOT is_tombstone AND normalized_evidence IS NOT NULL AND withdrawn_at IS NULL)
    );

CREATE UNIQUE INDEX source_evidence_one_current_identity_idx
    ON source_evidence_records (
        source_product, source_installation_id, source_record_type, source_record_id
    )
    WHERE is_current;

CREATE INDEX source_evidence_current_scope_observed_idx
    ON source_evidence_records (space_id, namespace_id, observed_at DESC)
    WHERE is_current AND NOT is_tombstone;

CREATE TABLE source_evidence_invalidations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_evidence_id UUID NOT NULL REFERENCES source_evidence_records(id) ON DELETE CASCADE,
    invalidated_source_evidence_id UUID NOT NULL REFERENCES source_evidence_records(id) ON DELETE CASCADE,
    target_kind VARCHAR(32) NOT NULL,
    target_id UUID,
    reason VARCHAR(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT source_evidence_invalidation_target_check CHECK (
        target_kind IN ('source_evidence', 'feedback_loop')
    ),
    CONSTRAINT source_evidence_invalidation_reason_check CHECK (
        reason IN ('superseded', 'withdrawn')
    )
);

-- Observation and Planning consumers use this view, never the revision log.
CREATE VIEW current_source_evidence AS
SELECT *
FROM source_evidence_records
WHERE is_current AND NOT is_tombstone;
