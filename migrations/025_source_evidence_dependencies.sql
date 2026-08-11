-- Bounded provenance edges for derived summaries and their exclusion state.
ALTER TABLE source_evidence_records
    ADD COLUMN is_stale BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE source_evidence_dependencies (
    derived_source_evidence_id UUID NOT NULL
        REFERENCES source_evidence_records(id) ON DELETE CASCADE,
    source_evidence_id UUID NOT NULL
        REFERENCES source_evidence_records(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (derived_source_evidence_id, source_evidence_id),
    CONSTRAINT source_evidence_dependency_not_self_check
        CHECK (derived_source_evidence_id <> source_evidence_id)
);

CREATE INDEX source_evidence_dependencies_source_idx
    ON source_evidence_dependencies (source_evidence_id);

ALTER TABLE source_evidence_invalidations
    DROP CONSTRAINT source_evidence_invalidation_target_check,
    ADD CONSTRAINT source_evidence_invalidation_target_check CHECK (
        target_kind IN (
            'source_evidence', 'feedback_loop', 'dependent_summary',
            'growth_model_input', 'plan'
        )
    );

DROP VIEW current_source_evidence;
CREATE VIEW current_source_evidence AS
SELECT *
FROM source_evidence_records
WHERE is_current AND NOT is_tombstone AND NOT is_stale;
