-- Space-scoped reminders for scheduled recall

ALTER TABLE reminders
    ADD COLUMN IF NOT EXISTS space_id UUID;

UPDATE reminders
SET space_id = memories.space_id
FROM memories
WHERE reminders.space_id IS NULL
  AND reminders.memory_id = memories.id;

UPDATE reminders
SET space_id = cognitive_spaces.id
FROM cognitive_spaces
WHERE reminders.space_id IS NULL
  AND reminders.user_id = cognitive_spaces.owner_user_id;

ALTER TABLE reminders
    ADD CONSTRAINT reminders_space_id_fkey
    FOREIGN KEY (space_id) REFERENCES cognitive_spaces(id) ON DELETE CASCADE;

ALTER TABLE reminders ALTER COLUMN space_id SET NOT NULL;

ALTER TABLE reminders
    ADD COLUMN IF NOT EXISTS title VARCHAR(500),
    ADD COLUMN IF NOT EXISTS status VARCHAR(50) NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS repeat_rule VARCHAR(50),
    ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ;

UPDATE reminders
SET status = CASE WHEN is_completed THEN 'completed' ELSE status END;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'reminders_status_check'
    ) THEN
        ALTER TABLE reminders
            ADD CONSTRAINT reminders_status_check
            CHECK (status IN ('pending', 'completed', 'cancelled'));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'reminders_repeat_rule_check'
    ) THEN
        ALTER TABLE reminders
            ADD CONSTRAINT reminders_repeat_rule_check
            CHECK (repeat_rule IS NULL OR repeat_rule IN ('daily', 'weekly', 'monthly'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_reminders_space_status_remind_at
    ON reminders(space_id, status, remind_at);
