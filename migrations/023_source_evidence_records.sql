-- Provider-neutral source evidence accepted through the authenticated Surface
-- Gateway. CognitiveSpace and Namespace come from the credential/request
-- boundary, never from the source envelope.
CREATE TABLE source_evidence_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL,
    namespace_id UUID NOT NULL,
    contract_version SMALLINT NOT NULL,
    source_product VARCHAR(64) NOT NULL,
    source_installation_id UUID NOT NULL,
    source_record_type VARCHAR(64) NOT NULL,
    source_record_id VARCHAR(128) NOT NULL,
    revision BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    evidence_trust VARCHAR(32) NOT NULL,
    provenance JSONB NOT NULL,
    normalized_evidence JSONB NOT NULL,
    payload_fingerprint CHAR(64) NOT NULL,
    acknowledgement JSONB,
    feedback_loop_id UUID,
    trace_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT source_evidence_identity_unique UNIQUE (
        source_product,
        source_installation_id,
        source_record_type,
        source_record_id,
        revision
    ),
    CONSTRAINT source_evidence_namespace_scope_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id) ON DELETE CASCADE,
    CONSTRAINT source_evidence_feedback_loop_scope_fkey
        FOREIGN KEY (feedback_loop_id, space_id, namespace_id)
        REFERENCES feedback_loops(id, space_id, namespace_id) ON DELETE CASCADE,
    CONSTRAINT source_evidence_trace_scope_fkey
        FOREIGN KEY (trace_id, space_id, namespace_id)
        REFERENCES traces(id, space_id, namespace_id) ON DELETE RESTRICT,
    CONSTRAINT source_evidence_contract_version_check CHECK (contract_version = 1),
    CONSTRAINT source_evidence_revision_check CHECK (revision > 0),
    CONSTRAINT source_evidence_time_order_check CHECK (occurred_at <= observed_at),
    CONSTRAINT source_evidence_trust_check CHECK (evidence_trust = 'contract_trusted'),
    CONSTRAINT source_evidence_fingerprint_check
        CHECK (payload_fingerprint ~ '^[0-9a-f]{64}$')
);

CREATE INDEX source_evidence_scope_observed_idx
    ON source_evidence_records (space_id, namespace_id, observed_at DESC);
