-- Trace evidence layer for local-first runtime metrics and feedback effectiveness.
-- Trace belongs to a Cognitive Space; Namespace remains a domain partition inside it.

CREATE TABLE IF NOT EXISTS traces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID,
    source_type VARCHAR(50) NOT NULL,
    task_type VARCHAR(50) NOT NULL,
    mode VARCHAR(50) NOT NULL,
    runtime VARCHAR(50) NOT NULL,
    input_summary TEXT,
    output_summary TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    latency_ms BIGINT,
    status VARCHAR(50) NOT NULL,
    model_provider VARCHAR(100),
    model_name VARCHAR(200),
    token_usage JSONB,
    estimated_cost_usd DOUBLE PRECISION,
    local_processing_ratio DOUBLE PRECISION,
    related_memory_ids UUID[] NOT NULL DEFAULT '{}',
    generated_memory_ids UUID[] NOT NULL DEFAULT '{}',
    generated_lens_run_ids UUID[] NOT NULL DEFAULT '{}',
    generated_review_report_ids UUID[] NOT NULL DEFAULT '{}',
    generated_feedback_loop_ids UUID[] NOT NULL DEFAULT '{}',
    user_feedback JSONB,
    error JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT traces_source_type_check
        CHECK (source_type IN ('http', 'cli', 'mcp', 'ui', 'background', 'test_fixture')),
    CONSTRAINT traces_task_type_check
        CHECK (task_type IN (
            'chat',
            'search',
            'lens_run',
            'review',
            'practice',
            'feedback',
            'planning',
            'install',
            'profile',
            'routing',
            'consolidation',
            'dreaming'
        )),
    CONSTRAINT traces_mode_check CHECK (mode IN ('fast', 'focused', 'deep', 'none')),
    CONSTRAINT traces_runtime_check
        CHECK (runtime IN ('local', 'cloud', 'hybrid', 'deterministic', 'unknown')),
    CONSTRAINT traces_status_check
        CHECK (status IN ('started', 'completed', 'failed', 'cancelled', 'skipped')),
    CONSTRAINT traces_latency_non_negative_check
        CHECK (latency_ms IS NULL OR latency_ms >= 0),
    CONSTRAINT traces_estimated_cost_non_negative_check
        CHECK (estimated_cost_usd IS NULL OR estimated_cost_usd >= 0),
    CONSTRAINT traces_local_processing_ratio_range_check
        CHECK (
            local_processing_ratio IS NULL
            OR (local_processing_ratio >= 0 AND local_processing_ratio <= 1)
        ),
    CONSTRAINT traces_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_traces_space_created
    ON traces(space_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_traces_space_namespace_created
    ON traces(space_id, namespace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_traces_task_status
    ON traces(task_type, status);
