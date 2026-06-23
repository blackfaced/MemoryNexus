-- Allow Observation Surface calls to write Trace provenance without modifying
-- the already-applied 014_traces.sql migration.

ALTER TABLE traces
    DROP CONSTRAINT IF EXISTS traces_task_type_check;

ALTER TABLE traces
    ADD CONSTRAINT traces_task_type_check
    CHECK (task_type IN (
        'chat',
        'capture',
        'search',
        'lens_run',
        'review',
        'practice',
        'feedback',
        'planning',
        'observation',
        'install',
        'profile',
        'routing',
        'consolidation',
        'dreaming'
    ));
