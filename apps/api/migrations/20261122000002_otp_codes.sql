-- One-time login codes. Only a keyed hash of the code is stored.
CREATE TABLE otp_codes (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    phone        text NOT NULL,                 -- E.164
    code_hash    bytea NOT NULL,
    attempts     integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    expires_at   timestamptz NOT NULL,
    consumed_at  timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX otp_codes_phone_created_idx ON otp_codes (phone, created_at DESC);

-- Emails are compared case-insensitively when linking Google accounts.
CREATE UNIQUE INDEX users_email_lower_idx ON users (lower(email)) WHERE email IS NOT NULL;
