-- Stable, provider-neutral retry identity for generic Performance outcomes.
-- Space remains the permission boundary; Namespace scopes the external event.
CREATE TABLE performance_idempotency_records (
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL,
    source_event_id VARCHAR(128) NOT NULL,
    payload_fingerprint CHAR(64) NOT NULL,
    feedback_loop_id UUID REFERENCES feedback_loops(id) ON DELETE CASCADE,
    trace_id UUID REFERENCES traces(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (space_id, namespace_id, source_event_id),
    CONSTRAINT performance_idempotency_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id)
        ON DELETE CASCADE,
    CONSTRAINT performance_idempotency_source_event_id_check
        CHECK (source_event_id ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
    CONSTRAINT performance_idempotency_fingerprint_check
        CHECK (payload_fingerprint ~ '^[0-9a-f]{64}$')
);
