-- Persisted periodic review reports as derived Lens interpretations

CREATE TABLE IF NOT EXISTS cognitive_review_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    lens_id UUID NOT NULL REFERENCES lenses(id) ON DELETE CASCADE,
    report_type VARCHAR(100) NOT NULL DEFAULT 'periodic_review',
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    report JSONB NOT NULL,
    source_memory_ids UUID[] NOT NULL DEFAULT '{}',
    source_lens_run_ids UUID[] NOT NULL DEFAULT '{}',
    summary_provider VARCHAR(100) NOT NULL,
    summary_source VARCHAR(100) NOT NULL,
    summary_model VARCHAR(200),
    summary_fallback_reason TEXT,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cognitive_review_reports_window_check CHECK (window_start < window_end)
);

CREATE INDEX IF NOT EXISTS idx_cognitive_review_reports_space_created
    ON cognitive_review_reports(space_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_cognitive_review_reports_lens_created
    ON cognitive_review_reports(lens_id, created_at DESC);
