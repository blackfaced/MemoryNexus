-- Lens minimal model.
-- A Lens is an interpretation strategy scoped to one Cognitive Space.

CREATE TABLE IF NOT EXISTS lenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    strategy VARCHAR(100) NOT NULL DEFAULT 'default',
    output_format VARCHAR(100) NOT NULL DEFAULT 'summary',
    retrieval_mode VARCHAR(100) NOT NULL DEFAULT 'semantic',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lenses_space_id ON lenses(space_id);
CREATE INDEX IF NOT EXISTS idx_lenses_created_by ON lenses(created_by);

CREATE TABLE IF NOT EXISTS lens_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    lens_id UUID NOT NULL REFERENCES lenses(id) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    query TEXT,
    input_memory_ids UUID[] NOT NULL DEFAULT '{}',
    output JSONB,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_lens_runs_lens_id ON lens_runs(lens_id);
CREATE INDEX IF NOT EXISTS idx_lens_runs_space_id ON lens_runs(space_id);
