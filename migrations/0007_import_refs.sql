ALTER TABLE products ADD COLUMN import_ref TEXT;
ALTER TABLE subscriptions ADD COLUMN import_ref TEXT;
ALTER TABLE services ADD COLUMN import_ref TEXT;
ALTER TABLE invoices ADD COLUMN import_ref TEXT;
ALTER TABLE tickets ADD COLUMN import_ref TEXT;

CREATE UNIQUE INDEX products_import_ref_idx ON products (import_ref) WHERE import_ref IS NOT NULL;
CREATE UNIQUE INDEX subscriptions_import_ref_idx ON subscriptions (import_ref) WHERE import_ref IS NOT NULL;
CREATE UNIQUE INDEX services_import_ref_idx ON services (import_ref) WHERE import_ref IS NOT NULL;
CREATE UNIQUE INDEX invoices_import_ref_idx ON invoices (import_ref) WHERE import_ref IS NOT NULL;
CREATE UNIQUE INDEX tickets_import_ref_idx ON tickets (import_ref) WHERE import_ref IS NOT NULL;

ALTER TABLE products DROP CONSTRAINT products_kind_check;
ALTER TABLE products ADD CONSTRAINT products_kind_check
    CHECK (kind IN ('gameserver', 'vps', 'volume', 'floating_ip', 'snapshot_storage', 'domain', 'webhosting', 'other'));
ALTER TABLE services DROP CONSTRAINT services_kind_check;
ALTER TABLE services ADD CONSTRAINT services_kind_check
    CHECK (kind IN ('gameserver', 'vps', 'webhosting', 'domain', 'other'));
