-- Family/shared Cognitive Space membership

ALTER TABLE cognitive_spaces
    ADD COLUMN IF NOT EXISTS space_type VARCHAR(50) NOT NULL DEFAULT 'personal';

ALTER TABLE cognitive_space_members
    ALTER COLUMN role SET DEFAULT 'viewer';

UPDATE cognitive_space_members
SET role = 'owner'
FROM cognitive_spaces
WHERE cognitive_space_members.space_id = cognitive_spaces.id
  AND cognitive_space_members.user_id = cognitive_spaces.owner_user_id;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'cognitive_space_members_role_check'
    ) THEN
        ALTER TABLE cognitive_space_members
            ADD CONSTRAINT cognitive_space_members_role_check
            CHECK (role IN ('owner', 'editor', 'viewer'));
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS cognitive_space_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    code VARCHAR(64) NOT NULL UNIQUE,
    role VARCHAR(50) NOT NULL DEFAULT 'viewer',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    accepted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cognitive_space_invites_role_check CHECK (role IN ('editor', 'viewer'))
);

CREATE INDEX IF NOT EXISTS idx_cognitive_space_invites_space
    ON cognitive_space_invites(space_id);

CREATE INDEX IF NOT EXISTS idx_cognitive_space_invites_code
    ON cognitive_space_invites(code);
