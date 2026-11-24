ALTER TABLE users
    ADD COLUMN default_mandate_id UUID REFERENCES payment_mandates (id) ON DELETE SET NULL,
    ADD COLUMN deleted_at TIMESTAMPTZ;
