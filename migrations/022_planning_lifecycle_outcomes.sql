-- Dated, typed owner-reported outcomes for a generic planning lifecycle.
-- This deliberately stores adherence semantics and provenance only; it does
-- not store health measurements or make an effectiveness judgement.
ALTER TABLE planning_lifecycles
    ADD CONSTRAINT planning_lifecycles_id_space_namespace_unique
    UNIQUE (id, space_id, namespace_id);

CREATE TABLE planning_lifecycle_outcomes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL,
    namespace_id UUID NOT NULL,
    lifecycle_id UUID NOT NULL,
    feedback_loop_id UUID NOT NULL,
    trace_id UUID NOT NULL,
    local_date DATE NOT NULL,
    action_id VARCHAR(128) NOT NULL,
    outcome VARCHAR(32) NOT NULL,
    source_event_id VARCHAR(128) NOT NULL,
    payload_fingerprint VARCHAR(64) NOT NULL,
    evidence_memory_id UUID,
    corrects_outcome_id UUID,
    superseded_by_outcome_id UUID,
    is_current BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT planning_lifecycle_outcomes_outcome_check
        CHECK (outcome IN ('performed', 'skipped', 'not_evaluable')),
    CONSTRAINT planning_lifecycle_outcomes_lifecycle_scope_fkey
        FOREIGN KEY (lifecycle_id, space_id, namespace_id)
        REFERENCES planning_lifecycles(id, space_id, namespace_id) ON DELETE CASCADE,
    CONSTRAINT planning_lifecycle_outcomes_feedback_loop_scope_fkey
        FOREIGN KEY (feedback_loop_id, space_id, namespace_id)
        REFERENCES feedback_loops(id, space_id, namespace_id) ON DELETE CASCADE,
    CONSTRAINT planning_lifecycle_outcomes_trace_scope_fkey
        FOREIGN KEY (trace_id, space_id, namespace_id)
        REFERENCES traces(id, space_id, namespace_id) ON DELETE RESTRICT,
    CONSTRAINT planning_lifecycle_outcomes_correction_fkey
        FOREIGN KEY (corrects_outcome_id) REFERENCES planning_lifecycle_outcomes(id) ON DELETE RESTRICT,
    CONSTRAINT planning_lifecycle_outcomes_superseded_fkey
        FOREIGN KEY (superseded_by_outcome_id) REFERENCES planning_lifecycle_outcomes(id) ON DELETE RESTRICT,
    CONSTRAINT planning_lifecycle_outcomes_event_scope_unique
        UNIQUE (space_id, namespace_id, source_event_id),
    CONSTRAINT planning_lifecycle_outcomes_decision_lineage_unique
        UNIQUE (id, space_id, namespace_id, lifecycle_id, feedback_loop_id, trace_id)
);
CREATE UNIQUE INDEX planning_lifecycle_outcomes_one_current_per_date
    ON planning_lifecycle_outcomes (lifecycle_id, local_date) WHERE is_current;

-- The composite foreign keys above prove each reference is in scope. This
-- trigger additionally proves that an outcome remains attached to the exact
-- lifecycle's FeedbackLoop and that a correction cannot be redirected across
-- a lifecycle, namespace, date, or already-superseded lineage.
CREATE OR REPLACE FUNCTION enforce_planning_lifecycle_outcome_lineage()
RETURNS TRIGGER AS $$
DECLARE
    lifecycle_feedback_loop_id UUID;
    corrected planning_lifecycle_outcomes%ROWTYPE;
BEGIN
    SELECT feedback_loop_id
      INTO lifecycle_feedback_loop_id
      FROM planning_lifecycles
     WHERE id = NEW.lifecycle_id
       AND space_id = NEW.space_id
       AND namespace_id = NEW.namespace_id;

    IF lifecycle_feedback_loop_id IS NULL
       OR lifecycle_feedback_loop_id <> NEW.feedback_loop_id THEN
        RAISE EXCEPTION 'planning lifecycle outcome feedback loop must match lifecycle scope';
    END IF;

    IF NEW.corrects_outcome_id IS NOT NULL THEN
        SELECT *
          INTO corrected
          FROM planning_lifecycle_outcomes
         WHERE id = NEW.corrects_outcome_id;

        IF NOT FOUND
           OR corrected.space_id <> NEW.space_id
           OR corrected.namespace_id <> NEW.namespace_id
           OR corrected.lifecycle_id <> NEW.lifecycle_id
           OR corrected.feedback_loop_id <> NEW.feedback_loop_id
           OR corrected.local_date <> NEW.local_date
           OR corrected.action_id <> NEW.action_id
           -- The application clears `is_current` before inserting the
           -- replacement to satisfy the partial unique index. Until the
           -- replacement is linked, `superseded_by_outcome_id` remains NULL.
           OR corrected.superseded_by_outcome_id IS NOT NULL THEN
            RAISE EXCEPTION 'planning lifecycle outcome correction target is invalid';
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER planning_lifecycle_outcomes_enforce_lineage
BEFORE INSERT OR UPDATE ON planning_lifecycle_outcomes
FOR EACH ROW EXECUTE FUNCTION enforce_planning_lifecycle_outcome_lineage();

CREATE TABLE planning_lifecycle_decisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL,
    namespace_id UUID NOT NULL,
    lifecycle_id UUID NOT NULL,
    feedback_loop_id UUID NOT NULL,
    outcome_id UUID NOT NULL,
    outcome_trace_id UUID NOT NULL,
    decision_trace_id UUID NOT NULL,
    disposition VARCHAR(32) NOT NULL,
    policy_version VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT planning_lifecycle_decisions_disposition_check
        CHECK (disposition IN ('continue', 'stop', 'retest')),
    CONSTRAINT planning_lifecycle_decisions_lifecycle_scope_fkey
        FOREIGN KEY (lifecycle_id, space_id, namespace_id)
        REFERENCES planning_lifecycles(id, space_id, namespace_id) ON DELETE CASCADE,
    CONSTRAINT planning_lifecycle_decisions_feedback_loop_scope_fkey
        FOREIGN KEY (feedback_loop_id, space_id, namespace_id)
        REFERENCES feedback_loops(id, space_id, namespace_id) ON DELETE CASCADE,
    CONSTRAINT planning_lifecycle_decisions_outcome_lineage_fkey
        FOREIGN KEY (outcome_id, space_id, namespace_id, lifecycle_id, feedback_loop_id, outcome_trace_id)
        REFERENCES planning_lifecycle_outcomes(id, space_id, namespace_id, lifecycle_id, feedback_loop_id, trace_id)
        ON DELETE RESTRICT,
    CONSTRAINT planning_lifecycle_decisions_decision_trace_scope_fkey
        FOREIGN KEY (decision_trace_id, space_id, namespace_id)
        REFERENCES traces(id, space_id, namespace_id) ON DELETE RESTRICT
);
