ALTER TABLE vps_members ADD COLUMN accepted_at TIMESTAMPTZ;

UPDATE vps_members SET accepted_at = created_at;
