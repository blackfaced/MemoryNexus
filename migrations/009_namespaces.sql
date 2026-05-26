-- Namespace minimal model.
-- A Namespace is a domain partition scoped inside one Cognitive Space.
-- Permissions remain based on cognitive_space_members through space_id.

CREATE TABLE IF NOT EXISTS namespaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    kind VARCHAR(50) NOT NULL,
    description TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (space_id, name),
    UNIQUE (id, space_id),
    CONSTRAINT namespaces_kind_check CHECK (kind IN ('reflective', 'skill')),
    CONSTRAINT namespaces_status_check CHECK (status IN ('active', 'archived'))
);

CREATE INDEX IF NOT EXISTS idx_namespaces_space_id ON namespaces(space_id);
CREATE INDEX IF NOT EXISTS idx_namespaces_created_by ON namespaces(created_by);
