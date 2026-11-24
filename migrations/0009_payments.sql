CREATE TABLE payment_customers (
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('mollie', 'stripe')),
    customer_ref TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, provider)
);

CREATE TABLE payment_mandates (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('mollie', 'stripe')),
    customer_ref TEXT NOT NULL,
    mandate_ref TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX payment_mandates_ref_idx ON payment_mandates (provider, mandate_ref);
CREATE INDEX payment_mandates_user_idx ON payment_mandates (user_id) WHERE revoked_at IS NULL;

CREATE TABLE payments (
    id UUID PRIMARY KEY,
    invoice_id BIGINT NOT NULL REFERENCES invoices (id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('mollie', 'stripe', 'bank_transfer', 'manual')),
    external_id TEXT,
    status TEXT NOT NULL CHECK (status IN ('open', 'paid', 'failed', 'cancelled', 'expired')),
    amount_cents INTEGER NOT NULL,
    method TEXT,
    checkout_url TEXT,
    recurring BOOLEAN NOT NULL DEFAULT FALSE,
    save_mandate BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    paid_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX payments_external_idx ON payments (provider, external_id) WHERE external_id IS NOT NULL;
CREATE INDEX payments_invoice_idx ON payments (invoice_id);

ALTER TABLE invoices ADD COLUMN reminder_sent_at TIMESTAMPTZ;
ALTER TABLE subscriptions ADD COLUMN mandate_id UUID REFERENCES payment_mandates (id) ON DELETE SET NULL;
ALTER TABLE subscriptions ADD COLUMN suspended_for_payment BOOLEAN NOT NULL DEFAULT FALSE;
