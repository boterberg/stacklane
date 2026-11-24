ALTER TABLE users ADD COLUMN tax_id_checked_at TIMESTAMPTZ;

DROP INDEX services_external_idx;
CREATE UNIQUE INDEX services_external_idx ON services (integration_id, external_id)
    WHERE external_id IS NOT NULL AND status <> 'terminated';
