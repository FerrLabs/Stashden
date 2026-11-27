CREATE TABLE browser_sign_ins (
    id            UUID PRIMARY KEY,
    challenge     TEXT        NOT NULL CHECK (length(challenge) = 43),
    device_name   TEXT        NOT NULL CHECK (char_length(device_name) BETWEEN 1 AND 100),
    loopback_port INTEGER     NOT NULL CHECK (loopback_port BETWEEN 1024 AND 65535),
    confirmation  TEXT        NOT NULL,
    user_id       UUID REFERENCES users (id) ON DELETE CASCADE,
    code_hash     TEXT,
    failures      INTEGER     NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    approved_at   TIMESTAMPTZ,
    expires_at    TIMESTAMPTZ NOT NULL,
    CHECK ((user_id IS NULL) = (code_hash IS NULL) AND (user_id IS NULL) = (approved_at IS NULL))
);

CREATE INDEX browser_sign_ins_by_expiry ON browser_sign_ins (expires_at);
