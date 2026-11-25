CREATE TABLE brand_assets (
    kind TEXT PRIMARY KEY CHECK (kind IN ('logo', 'logo_dark', 'favicon')),
    content_type TEXT NOT NULL,
    data BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
