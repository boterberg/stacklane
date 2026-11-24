CREATE TABLE vps_firewalls (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    auto_assign BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX vps_firewalls_user ON vps_firewalls (user_id);

CREATE TABLE vps_firewall_rules (
    id UUID PRIMARY KEY,
    firewall_id UUID NOT NULL REFERENCES vps_firewalls (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('in', 'out')),
    action TEXT NOT NULL CHECK (action IN ('ACCEPT', 'DROP', 'REJECT')),
    proto TEXT,
    dport TEXT,
    source TEXT,
    comment TEXT
);

CREATE INDEX vps_firewall_rules_firewall ON vps_firewall_rules (firewall_id, position);

CREATE TABLE vps_firewall_servers (
    firewall_id UUID NOT NULL REFERENCES vps_firewalls (id) ON DELETE CASCADE,
    service_id UUID NOT NULL REFERENCES services (id) ON DELETE CASCADE,
    PRIMARY KEY (firewall_id, service_id)
);
