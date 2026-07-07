-- Namespace Knowledge Refresh V1 bounded persistence.
-- External adapters own discovery/fetching/extraction; MemoryNexus stores only
-- scoped contract fields needed for validation, provenance, and Observation.

CREATE TABLE knowledge_acquisition_traces (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    submitted_by TEXT NOT NULL,
    acquisition_kind TEXT NOT NULL CHECK (
        acquisition_kind IN (
            'source_candidate',
            'source_policy_review',
            'knowledge_context',
            'revalidation'
        )
    ),
    discovery_method TEXT NOT NULL,
    extraction_method TEXT NOT NULL,
    private_context_used BOOLEAN NOT NULL DEFAULT FALSE,
    private_context_basis JSONB,
    opt_in_proof JSONB,
    source_handles JSONB NOT NULL DEFAULT '[]'::jsonb,
    source_observed_at TIMESTAMPTZ NOT NULL,
    extraction_run_id TEXT,
    tool_or_adapter_version TEXT,
    validation_summary JSONB NOT NULL DEFAULT '{}'::jsonb,
    redacted_diagnostics JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (jsonb_typeof(source_handles) = 'array'),
    CHECK (jsonb_typeof(validation_summary) = 'object'),
    CHECK (jsonb_typeof(redacted_diagnostics) = 'object'),
    CHECK (jsonb_typeof(metadata) = 'object'),
    CHECK (private_context_used = FALSE OR opt_in_proof IS NOT NULL)
);

CREATE INDEX knowledge_acquisition_traces_scope_idx
    ON knowledge_acquisition_traces(space_id, namespace_id, created_at DESC);

CREATE TABLE knowledge_source_candidates (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('proposed', 'approved', 'rejected', 'expired')),
    proposed_source JSONB NOT NULL,
    proposed_use TEXT NOT NULL,
    proposer TEXT NOT NULL,
    acquisition_trace_id UUID NOT NULL REFERENCES knowledge_acquisition_traces(id) ON DELETE RESTRICT,
    private_context_used BOOLEAN NOT NULL DEFAULT FALSE,
    opt_in_proof JSONB,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    quality_signals JSONB NOT NULL DEFAULT '{}'::jsonb,
    freshness JSONB NOT NULL DEFAULT '{}'::jsonb,
    expiry TIMESTAMPTZ NOT NULL,
    downstream_link_candidates JSONB NOT NULL DEFAULT '[]'::jsonb,
    decision JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (jsonb_typeof(proposed_source) = 'object'),
    CHECK (jsonb_typeof(provenance) = 'object'),
    CHECK (jsonb_typeof(quality_signals) = 'object'),
    CHECK (jsonb_typeof(freshness) = 'object'),
    CHECK (jsonb_typeof(downstream_link_candidates) = 'array'),
    CHECK (jsonb_typeof(metadata) = 'object'),
    CHECK (private_context_used = FALSE OR opt_in_proof IS NOT NULL)
);

CREATE INDEX knowledge_source_candidates_scope_state_idx
    ON knowledge_source_candidates(space_id, namespace_id, state, updated_at DESC);

CREATE TABLE knowledge_source_policies (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('active', 'paused', 'revoked', 'expired')),
    source_candidate_id UUID NOT NULL REFERENCES knowledge_source_candidates(id) ON DELETE RESTRICT,
    source_descriptor JSONB NOT NULL,
    allowed_use JSONB NOT NULL DEFAULT '[]'::jsonb,
    disallowed_use JSONB NOT NULL DEFAULT '[]'::jsonb,
    privacy_policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    refresh_policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    quality_thresholds JSONB NOT NULL DEFAULT '{}'::jsonb,
    freshness_requirements JSONB NOT NULL DEFAULT '{}'::jsonb,
    expiry TIMESTAMPTZ NOT NULL,
    approved_by TEXT NOT NULL,
    approved_at TIMESTAMPTZ NOT NULL,
    revoked_or_paused_reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (jsonb_typeof(source_descriptor) = 'object'),
    CHECK (jsonb_typeof(allowed_use) = 'array'),
    CHECK (jsonb_typeof(disallowed_use) = 'array'),
    CHECK (jsonb_typeof(privacy_policy) = 'object'),
    CHECK (jsonb_typeof(refresh_policy) = 'object'),
    CHECK (jsonb_typeof(quality_thresholds) = 'object'),
    CHECK (jsonb_typeof(freshness_requirements) = 'object'),
    CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX knowledge_source_policies_scope_state_idx
    ON knowledge_source_policies(space_id, namespace_id, state, updated_at DESC);

CREATE TABLE knowledge_contexts (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    source_policy_id UUID NOT NULL REFERENCES knowledge_source_policies(id) ON DELETE RESTRICT,
    source_candidate_id UUID NOT NULL REFERENCES knowledge_source_candidates(id) ON DELETE RESTRICT,
    acquisition_trace_id UUID NOT NULL REFERENCES knowledge_acquisition_traces(id) ON DELETE RESTRICT,
    state TEXT NOT NULL CHECK (state IN ('candidate', 'valid', 'rejected', 'expired')),
    context_type TEXT NOT NULL CHECK (
        context_type IN (
            'reference_claims',
            'rubric_context',
            'practice_context',
            'trend_context',
            'contradiction_context',
            'review_context'
        )
    ),
    structured_claims JSONB NOT NULL DEFAULT '[]'::jsonb,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    quality_signals JSONB NOT NULL DEFAULT '{}'::jsonb,
    freshness JSONB NOT NULL DEFAULT '{}'::jsonb,
    expiry TIMESTAMPTZ NOT NULL,
    evidence_snippets JSONB NOT NULL DEFAULT '[]'::jsonb,
    private_context_used BOOLEAN NOT NULL DEFAULT FALSE,
    opt_in_proof JSONB,
    downstream_links JSONB NOT NULL DEFAULT '[]'::jsonb,
    conflict_notes JSONB NOT NULL DEFAULT '[]'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (jsonb_typeof(structured_claims) = 'array'),
    CHECK (jsonb_typeof(provenance) = 'object'),
    CHECK (jsonb_typeof(quality_signals) = 'object'),
    CHECK (jsonb_typeof(freshness) = 'object'),
    CHECK (jsonb_typeof(evidence_snippets) = 'array'),
    CHECK (jsonb_typeof(downstream_links) = 'array'),
    CHECK (jsonb_typeof(conflict_notes) = 'array'),
    CHECK (jsonb_typeof(metadata) = 'object'),
    CHECK (private_context_used = FALSE OR opt_in_proof IS NOT NULL)
);

CREATE INDEX knowledge_contexts_scope_state_idx
    ON knowledge_contexts(space_id, namespace_id, state, updated_at DESC);
