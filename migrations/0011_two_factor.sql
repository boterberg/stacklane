ALTER TABLE users
    ADD COLUMN totp_secret TEXT,
    ADD COLUMN totp_pending TEXT,
    ADD COLUMN totp_last_step BIGINT,
    ADD COLUMN two_factor_enabled_at TIMESTAMPTZ;

CREATE TABLE recovery_codes (
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    used_at TIMESTAMPTZ,
    PRIMARY KEY (user_id, code_hash)
);
