CREATE TABLE service_activity (
    id UUID PRIMARY KEY,
    service_id UUID NOT NULL REFERENCES services (id) ON DELETE CASCADE,
    user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    detail TEXT,
    ip TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX service_activity_service_idx ON service_activity (service_id, created_at DESC);
