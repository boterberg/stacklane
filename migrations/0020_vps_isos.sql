CREATE TABLE vps_isos (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    integration_id UUID NOT NULL REFERENCES integrations (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    firmware TEXT NOT NULL CHECK (firmware IN ('bios', 'uefi')),
    status TEXT NOT NULL CHECK (status IN ('importing', 'ready', 'failed')),
    external_id TEXT,
    last_error TEXT,
    mounted_on UUID REFERENCES services (id) ON DELETE SET NULL,
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX vps_isos_user ON vps_isos (user_id);
CREATE UNIQUE INDEX vps_isos_mounted ON vps_isos (mounted_on) WHERE mounted_on IS NOT NULL;
