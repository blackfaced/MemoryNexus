-- Minimal in-app reminder delivery channel and provenance

ALTER TABLE reminders
    ADD COLUMN IF NOT EXISTS delivery_channel VARCHAR(50) NOT NULL DEFAULT 'in_app',
    ADD COLUMN IF NOT EXISTS delivery_status VARCHAR(50) NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS delivery_attempted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS delivered_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS delivery_error TEXT,
    ADD COLUMN IF NOT EXISTS delivery_provenance JSONB NOT NULL DEFAULT '{}'::jsonb;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'reminders_delivery_channel_check'
    ) THEN
        ALTER TABLE reminders
            ADD CONSTRAINT reminders_delivery_channel_check
            CHECK (delivery_channel IN ('in_app'));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'reminders_delivery_status_check'
    ) THEN
        ALTER TABLE reminders
            ADD CONSTRAINT reminders_delivery_status_check
            CHECK (delivery_status IN ('pending', 'delivered', 'failed'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_reminders_space_delivery_status_remind_at
    ON reminders(space_id, delivery_channel, delivery_status, remind_at);
