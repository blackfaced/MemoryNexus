-- Persisted profile projections for personal agents and UI clients

CREATE TABLE IF NOT EXISTS cognitive_profile_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    lens_id UUID REFERENCES lenses(id) ON DELETE SET NULL,
    target VARCHAR(50) NOT NULL DEFAULT 'llm_context',
    profile JSONB NOT NULL,
    source_memory_ids UUID[] NOT NULL DEFAULT '{}',
    source_lens_run_ids UUID[] NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cognitive_profile_snapshots_space_created
    ON cognitive_profile_snapshots(space_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_cognitive_profile_snapshots_created_by
    ON cognitive_profile_snapshots(created_by);
