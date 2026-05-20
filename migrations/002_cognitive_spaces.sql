-- Cognitive Space ownership boundary

CREATE TABLE IF NOT EXISTS cognitive_spaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(200) NOT NULL,
    description TEXT,
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    default_lens_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cognitive_space_members (
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'owner',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (space_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_cognitive_spaces_owner ON cognitive_spaces(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_cognitive_space_members_user ON cognitive_space_members(user_id);

WITH inserted_spaces AS (
    INSERT INTO cognitive_spaces (name, owner_user_id)
    SELECT username || ' Personal Space', id
    FROM users
    WHERE NOT EXISTS (
        SELECT 1 FROM cognitive_spaces WHERE cognitive_spaces.owner_user_id = users.id
    )
    RETURNING id, owner_user_id
)
INSERT INTO cognitive_space_members (space_id, user_id, role)
SELECT id, owner_user_id, 'owner'
FROM inserted_spaces
ON CONFLICT (space_id, user_id) DO NOTHING;

ALTER TABLE memories ADD COLUMN IF NOT EXISTS space_id UUID;

UPDATE memories
SET space_id = cognitive_spaces.id
FROM cognitive_spaces
WHERE memories.space_id IS NULL
  AND memories.user_id = cognitive_spaces.owner_user_id;

ALTER TABLE memories
    ADD CONSTRAINT memories_space_id_fkey
    FOREIGN KEY (space_id) REFERENCES cognitive_spaces(id) ON DELETE CASCADE;

ALTER TABLE memories ALTER COLUMN space_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_memories_space ON memories(space_id);
CREATE INDEX IF NOT EXISTS idx_memories_space_created ON memories(space_id, created_at DESC);
