ALTER TABLE services DROP CONSTRAINT services_kind_check;
ALTER TABLE services ADD CONSTRAINT services_kind_check
    CHECK (kind IN ('gameserver', 'vps', 'webhosting', 'domain', 'volume', 'floating_ip', 'other'));
