ALTER TABLE integrations DROP CONSTRAINT integrations_kind_check;
ALTER TABLE integrations ADD CONSTRAINT integrations_kind_check
    CHECK (kind IN ('pterodactyl', 'proxmox', 'openstack', 'virtfusion', 'directadmin', 'plesk', 'openprovider', 'cpanel'));
