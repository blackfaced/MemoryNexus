-- Generic structured planning lifecycle. The owning namespace policy supplies
-- action semantics; this table only provides stable lifecycle/replay identity.
ALTER TABLE feedback_loops
    ADD CONSTRAINT feedback_loops_id_space_namespace_unique
    UNIQUE (id, space_id, namespace_id);

CREATE TABLE planning_lifecycles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    namespace_id UUID NOT NULL,
    feedback_loop_id UUID NOT NULL,
    planning_trace_id UUID UNIQUE REFERENCES traces(id) ON DELETE RESTRICT,
    policy_version VARCHAR(128) NOT NULL,
    action_id VARCHAR(128) NOT NULL,
    action JSONB NOT NULL,
    selected_evidence_ids JSONB NOT NULL,
    expected_signal VARCHAR(512) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT planning_lifecycles_namespace_same_space_fkey
        FOREIGN KEY (namespace_id, space_id) REFERENCES namespaces(id, space_id) ON DELETE CASCADE,
    CONSTRAINT planning_lifecycles_feedback_loop_same_scope_fkey
        FOREIGN KEY (feedback_loop_id, space_id, namespace_id)
        REFERENCES feedback_loops(id, space_id, namespace_id) ON DELETE CASCADE,
    CONSTRAINT planning_lifecycles_active_status_check CHECK (status IN ('active', 'completed', 'cancelled'))
);
CREATE UNIQUE INDEX planning_lifecycles_one_active_per_namespace
    ON planning_lifecycles (space_id, namespace_id) WHERE status = 'active';
