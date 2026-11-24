ALTER TABLE integrations DROP CONSTRAINT integrations_kind_check;
ALTER TABLE integrations ADD CONSTRAINT integrations_kind_check
    CHECK (kind IN ('pterodactyl', 'proxmox', 'openstack', 'virtfusion', 'openprovider'));

ALTER TABLE subscription_items ADD COLUMN period_price_cents INTEGER;

CREATE TABLE domain_extensions (
    id UUID PRIMARY KEY,
    integration_id UUID NOT NULL REFERENCES integrations (id) ON DELETE CASCADE,
    extension TEXT NOT NULL UNIQUE,
    register_cents INTEGER NOT NULL CHECK (register_cents >= 0),
    renew_cents INTEGER NOT NULL CHECK (renew_cents >= 0),
    transfer_cents INTEGER NOT NULL CHECK (transfer_cents >= 0),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE whois_profiles (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    company TEXT,
    email TEXT NOT NULL,
    phone TEXT NOT NULL,
    street TEXT NOT NULL,
    house_number TEXT NOT NULL,
    zipcode TEXT NOT NULL,
    city TEXT NOT NULL,
    country TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX whois_profiles_user ON whois_profiles (user_id);

CREATE TABLE whois_profile_handles (
    whois_profile_id UUID NOT NULL REFERENCES whois_profiles (id) ON DELETE CASCADE,
    integration_id UUID NOT NULL REFERENCES integrations (id) ON DELETE CASCADE,
    handle TEXT NOT NULL,
    PRIMARY KEY (whois_profile_id, integration_id)
);

CREATE UNIQUE INDEX services_active_domain ON services (lower(name))
    WHERE kind = 'domain' AND status NOT IN ('terminated', 'failed');
