CREATE TABLE vps_networks (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    integration_id UUID NOT NULL REFERENCES integrations (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    cidr TEXT NOT NULL,
    vlan INTEGER,
    external_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX vps_networks_user ON vps_networks (user_id);
CREATE UNIQUE INDEX vps_networks_name ON vps_networks (user_id, lower(name));
CREATE UNIQUE INDEX vps_networks_vlan ON vps_networks (integration_id, vlan) WHERE vlan IS NOT NULL;

CREATE TABLE vps_network_servers (
    network_id UUID NOT NULL REFERENCES vps_networks (id) ON DELETE CASCADE,
    service_id UUID NOT NULL REFERENCES services (id) ON DELETE CASCADE,
    address TEXT NOT NULL,
    port TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (network_id, service_id)
);

CREATE UNIQUE INDEX vps_network_servers_address ON vps_network_servers (network_id, address);
CREATE INDEX vps_network_servers_service ON vps_network_servers (service_id);
