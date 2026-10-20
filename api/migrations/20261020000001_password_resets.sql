ALTER TYPE attempt_scope ADD VALUE 'reset';

CREATE TABLE password_resets (
    hash       TEXT PRIMARY KEY,
    user_id    UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX password_resets_by_expiry ON password_resets (expires_at);
