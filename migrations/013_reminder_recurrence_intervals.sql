-- Allow the supported recurrence-rule subset with explicit intervals.
--
-- Accepted forms:
-- - daily, weekly, monthly
-- - daily:<positive integer>, weekly:<positive integer>, monthly:<positive integer>

ALTER TABLE reminders
    DROP CONSTRAINT IF EXISTS reminders_repeat_rule_check;

ALTER TABLE reminders
    ADD CONSTRAINT reminders_repeat_rule_check
    CHECK (
        repeat_rule IS NULL
        OR repeat_rule ~ '^(daily|weekly|monthly)(:[1-9][0-9]*)?$'
    );
