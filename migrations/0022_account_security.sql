ALTER TABLE users
    ADD COLUMN email_verified_at TIMESTAMPTZ,
    ADD COLUMN imported_at TIMESTAMPTZ,
    ADD COLUMN second_factor_failures INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN second_factor_locked_until TIMESTAMPTZ;

UPDATE users SET email_verified_at = created_at WHERE role = 'admin';

CREATE TABLE email_verifications (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    email CITEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX email_verifications_user_id_idx ON email_verifications (user_id);
