CREATE TABLE products (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('gameserver', 'vps', 'volume', 'floating_ip', 'snapshot_storage', 'domain', 'webhosting')),
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    specs JSONB NOT NULL DEFAULT '{}'::jsonb,
    monthly_price_cents INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE subscriptions (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'active', 'suspended', 'cancelled', 'expired')),
    billing_interval TEXT NOT NULL CHECK (billing_interval IN ('monthly', 'quarterly', 'half_yearly', 'yearly')),
    payment_mode TEXT NOT NULL DEFAULT 'manual' CHECK (payment_mode IN ('manual', 'automatic')),
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX subscriptions_user_id_idx ON subscriptions (user_id);

CREATE TABLE subscription_items (
    id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES subscriptions (id) ON DELETE CASCADE,
    product_id UUID REFERENCES products (id),
    description TEXT NOT NULL,
    specs_line TEXT,
    monthly_price_cents INTEGER NOT NULL
);

CREATE TABLE invoices (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('open', 'paid', 'cancelled', 'refunded')),
    issued_at DATE NOT NULL DEFAULT CURRENT_DATE,
    due_at DATE NOT NULL DEFAULT CURRENT_DATE,
    paid_at TIMESTAMPTZ,
    tax_rate_basis_points INTEGER NOT NULL DEFAULT 2100,
    total_cents INTEGER NOT NULL,
    tax_cents INTEGER NOT NULL,
    billing_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER SEQUENCE invoices_id_seq RESTART WITH 100001;

CREATE INDEX invoices_user_id_idx ON invoices (user_id);

CREATE TABLE invoice_lines (
    id UUID PRIMARY KEY,
    invoice_id BIGINT NOT NULL REFERENCES invoices (id) ON DELETE CASCADE,
    subscription_id UUID REFERENCES subscriptions (id) ON DELETE SET NULL,
    description TEXT NOT NULL,
    period_start DATE,
    period_end DATE,
    amount_cents INTEGER NOT NULL
);
