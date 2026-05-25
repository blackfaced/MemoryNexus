-- Scope user tags by user_id instead of a global tag name.

ALTER TABLE tags
DROP CONSTRAINT IF EXISTS tags_name_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_user_name_unique
ON tags(user_id, name)
WHERE user_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_system_name_unique
ON tags(name)
WHERE user_id IS NULL;
