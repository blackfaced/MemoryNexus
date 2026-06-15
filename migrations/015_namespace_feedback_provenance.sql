-- Thread Namespace and FeedbackLoop provenance through existing derived objects.
-- 014 is intentionally left available for #99 Trace schema foundation.

ALTER TABLE memories
    ADD COLUMN IF NOT EXISTS namespace_id UUID,
    ADD COLUMN IF NOT EXISTS feedback_loop_id UUID;

ALTER TABLE lenses
    ADD COLUMN IF NOT EXISTS namespace_id UUID;

ALTER TABLE lens_runs
    ADD COLUMN IF NOT EXISTS namespace_id UUID,
    ADD COLUMN IF NOT EXISTS feedback_loop_id UUID;

ALTER TABLE cognitive_review_reports
    ADD COLUMN IF NOT EXISTS namespace_id UUID,
    ADD COLUMN IF NOT EXISTS feedback_loop_id UUID;

ALTER TABLE cognitive_profile_snapshots
    ADD COLUMN IF NOT EXISTS namespace_id UUID,
    ADD COLUMN IF NOT EXISTS feedback_loop_id UUID;

ALTER TABLE feedback_loops
    ADD CONSTRAINT feedback_loops_id_space_unique UNIQUE (id, space_id);

ALTER TABLE memories
    ADD CONSTRAINT memories_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id),
    ADD CONSTRAINT memories_feedback_loop_same_space_fkey
        FOREIGN KEY (feedback_loop_id, space_id)
        REFERENCES feedback_loops(id, space_id);

ALTER TABLE lenses
    ADD CONSTRAINT lenses_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id);

ALTER TABLE lens_runs
    ADD CONSTRAINT lens_runs_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id),
    ADD CONSTRAINT lens_runs_feedback_loop_same_space_fkey
        FOREIGN KEY (feedback_loop_id, space_id)
        REFERENCES feedback_loops(id, space_id);

ALTER TABLE cognitive_review_reports
    ADD CONSTRAINT cognitive_review_reports_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id),
    ADD CONSTRAINT cognitive_review_reports_feedback_loop_same_space_fkey
        FOREIGN KEY (feedback_loop_id, space_id)
        REFERENCES feedback_loops(id, space_id);

ALTER TABLE cognitive_profile_snapshots
    ADD CONSTRAINT cognitive_profile_snapshots_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id)
        REFERENCES namespaces(id, space_id),
    ADD CONSTRAINT cognitive_profile_snapshots_feedback_loop_same_space_fkey
        FOREIGN KEY (feedback_loop_id, space_id)
        REFERENCES feedback_loops(id, space_id);

CREATE INDEX IF NOT EXISTS idx_memories_space_namespace_created
    ON memories(space_id, namespace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_memories_space_feedback_loop_created
    ON memories(space_id, feedback_loop_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_lenses_space_namespace
    ON lenses(space_id, namespace_id);

CREATE INDEX IF NOT EXISTS idx_lens_runs_space_namespace_created
    ON lens_runs(space_id, namespace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_review_reports_space_namespace_created
    ON cognitive_review_reports(space_id, namespace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_profile_snapshots_space_namespace_created
    ON cognitive_profile_snapshots(space_id, namespace_id, created_at DESC);
