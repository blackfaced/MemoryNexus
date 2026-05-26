-- Minimal FeedbackLoop model scoped inside a Cognitive Space and Namespace.

CREATE TABLE IF NOT EXISTS feedback_loops (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL,
    goal TEXT NOT NULL,
    task TEXT NOT NULL,
    attempt TEXT,
    evaluation TEXT,
    feedback TEXT,
    adjustment TEXT,
    next_task TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT feedback_loops_status_check CHECK (status IN ('active', 'completed', 'paused')),
    CONSTRAINT feedback_loops_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_feedback_loops_space_namespace_status
    ON feedback_loops(space_id, namespace_id, status);

CREATE INDEX IF NOT EXISTS idx_feedback_loops_created_at
    ON feedback_loops(created_at DESC);
