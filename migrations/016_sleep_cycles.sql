-- SleepCycle records one offline consolidation lifecycle over a bounded evidence window.
-- It stores links only; consolidation, Dreaming, scheduling, and AI calls are separate concerns.

CREATE TABLE IF NOT EXISTS sleep_cycles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID,
    cycle_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    evidence_window_start TIMESTAMPTZ NOT NULL,
    evidence_window_end TIMESTAMPTZ NOT NULL,
    input_trace_ids UUID[] NOT NULL DEFAULT '{}',
    input_memory_ids UUID[] NOT NULL DEFAULT '{}',
    input_feedback_loop_ids UUID[] NOT NULL DEFAULT '{}',
    input_review_report_ids UUID[] NOT NULL DEFAULT '{}',
    generated_memory_ids UUID[] NOT NULL DEFAULT '{}',
    triggering_trace_id UUID,
    error TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sleep_cycles_cycle_type_check CHECK (cycle_type IN ('daily', 'weekly', 'manual')),
    CONSTRAINT sleep_cycles_status_check CHECK (
        status IN ('pending', 'running', 'completed', 'failed', 'cancelled')
    ),
    CONSTRAINT sleep_cycles_window_check CHECK (evidence_window_start < evidence_window_end),
    CONSTRAINT sleep_cycles_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sleep_cycles_space_status
    ON sleep_cycles(space_id, status);

CREATE INDEX IF NOT EXISTS idx_sleep_cycles_space_namespace_window
    ON sleep_cycles(space_id, namespace_id, evidence_window_start, evidence_window_end);

CREATE INDEX IF NOT EXISTS idx_sleep_cycles_created_at
    ON sleep_cycles(created_at DESC);
