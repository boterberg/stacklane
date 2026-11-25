CREATE TABLE gameserver_members (
    id UUID PRIMARY KEY,
    service_id UUID NOT NULL REFERENCES services (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    permissions TEXT[] NOT NULL,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (service_id, user_id)
);

CREATE INDEX gameserver_members_user_idx ON gameserver_members (user_id);
