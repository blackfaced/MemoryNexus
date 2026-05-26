-- Persist source provenance for memories created from derived inputs such as voice capture.

ALTER TABLE memories
    ADD COLUMN IF NOT EXISTS source_type VARCHAR(100) NOT NULL DEFAULT 'manual',
    ADD COLUMN IF NOT EXISTS source_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_memories_source_type ON memories(source_type);
