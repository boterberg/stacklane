CREATE TABLE integrations (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('pterodactyl', 'proxmox', 'openstack', 'virtfusion')),
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    credentials BYTEA NOT NULL,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_checked_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE products ADD COLUMN description TEXT;
ALTER TABLE products ADD COLUMN integration_id UUID REFERENCES integrations (id) ON DELETE SET NULL;
ALTER TABLE products ADD COLUMN provisioning JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE services (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    product_id UUID REFERENCES products (id) ON DELETE SET NULL,
    subscription_id UUID REFERENCES subscriptions (id) ON DELETE SET NULL,
    integration_id UUID REFERENCES integrations (id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK (kind IN ('gameserver', 'vps', 'webhosting', 'domain')),
    name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'provisioning', 'active', 'suspended', 'terminated', 'failed')),
    external_id TEXT,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX services_user_id_idx ON services (user_id);
CREATE UNIQUE INDEX services_external_idx ON services (integration_id, external_id) WHERE external_id IS NOT NULL;
