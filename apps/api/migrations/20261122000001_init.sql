-- Voclara initial schema.
-- Conventions: uuid primary keys, timestamptz everywhere, durations in integer
-- seconds, money in minor units (paisa/cents). Deleting a user cascades to all
-- of their data (the "delete account" button must really delete).

-- ---------------------------------------------------------------- users & auth

CREATE TABLE users (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text UNIQUE,
    phone         text UNIQUE,                 -- E.164, e.g. +923001234567
    google_sub    text UNIQUE,                 -- Google account id
    name          text,
    l1            text NOT NULL DEFAULT 'ur' CHECK (l1 IN ('ur', 'hi', 'bn', 'ar', 'en')),
    level         text CHECK (level IN ('a1', 'a2', 'b1', 'b2', 'c1', 'c2')),
    goal          text CHECK (goal IN ('ielts', 'interview', 'work', 'abroad', 'daily')),
    country       char(2),                     -- ISO 3166-1, drives pricing
    timezone      text NOT NULL DEFAULT 'Asia/Karachi',
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CHECK (email IS NOT NULL OR phone IS NOT NULL OR google_sub IS NOT NULL)
);

-- Rotating refresh tokens. Only a hash is stored. Reusing a token that was
-- already replaced revokes its whole family.
CREATE TABLE refresh_tokens (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id    uuid NOT NULL,
    token_hash   bytea NOT NULL UNIQUE,
    expires_at   timestamptz NOT NULL,
    revoked_at   timestamptz,
    replaced_by  uuid REFERENCES refresh_tokens(id) ON DELETE SET NULL,
    user_agent   text,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX refresh_tokens_user_idx   ON refresh_tokens (user_id);
CREATE INDEX refresh_tokens_family_idx ON refresh_tokens (family_id);

-- ---------------------------------------------------------------- sessions

CREATE TABLE sessions (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mode             text NOT NULL CHECK (mode IN ('talk', 'mock', 'interview', 'drill')),
    topic            text,
    provider         text NOT NULL,             -- gemini | openai | pipeline
    model            text,
    prompt_version   text NOT NULL,
    status           text NOT NULL DEFAULT 'active'
                     CHECK (status IN ('active', 'completed', 'expired', 'failed')),
    max_secs         integer NOT NULL CHECK (max_secs > 0),  -- token TTL at start
    seconds_used     integer CHECK (seconds_used >= 0),
    transcript       jsonb,
    audio_key        text,                      -- R2 object key
    audio_delete_at  timestamptz,               -- 30-day retention
    started_at       timestamptz NOT NULL DEFAULT now(),
    ended_at         timestamptz
);
CREATE INDEX sessions_user_started_idx ON sessions (user_id, started_at DESC);
CREATE INDEX sessions_audio_delete_idx ON sessions (audio_delete_at) WHERE audio_key IS NOT NULL;

CREATE TABLE session_costs (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id   uuid NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    provider     text NOT NULL,                 -- gemini | openai | azure | grader
    input_secs   integer NOT NULL DEFAULT 0,
    output_secs  integer NOT NULL DEFAULT 0,
    tokens       jsonb NOT NULL DEFAULT '{}',
    usd          numeric(12, 6) NOT NULL DEFAULT 0,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX session_costs_session_idx ON session_costs (session_id);
CREATE INDEX session_costs_created_idx ON session_costs (created_at);

-- ---------------------------------------------------------------- scoring

-- IELTS-style bands: 0-9 in 0.5 steps.
CREATE DOMAIN band AS numeric(3, 1)
    CHECK (VALUE >= 0 AND VALUE <= 9 AND (VALUE * 2) = trunc(VALUE * 2));

CREATE TABLE scores (
    session_id      uuid PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    fluency         band,
    lexical         band,
    grammar         band,
    pronunciation   band,
    overall         band,
    evidence        jsonb NOT NULL DEFAULT '{}',
    corrections     jsonb NOT NULL DEFAULT '[]',  -- [{said, better, why_en, why_l1}]
    fillers         jsonb NOT NULL DEFAULT '{}',
    wpm             integer,
    next_focus      text,
    grader_version  text NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE pronunciation_results (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id     uuid NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    utterance_idx  integer NOT NULL,
    accuracy       real,
    fluency        real,
    prosody        real,
    completeness   real,
    words          jsonb NOT NULL DEFAULT '[]',
    created_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (session_id, utterance_idx)
);

-- ---------------------------------------------------------------- billing

CREATE TABLE purchases (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider      text NOT NULL CHECK (provider IN
                  ('google_play', 'revenuecat', 'safepay', 'jazzcash', 'easypaisa',
                   'paddle', 'lemonsqueezy', 'manual')),
    provider_ref  text NOT NULL,                -- order id / transaction id
    product_id    text NOT NULL,
    kind          text NOT NULL CHECK (kind IN ('subscription', 'minute_pack', 'mock_pack')),
    amount_minor  bigint NOT NULL CHECK (amount_minor >= 0),
    currency      char(3) NOT NULL,
    status        text NOT NULL CHECK (status IN ('pending', 'paid', 'refunded', 'failed')),
    raw           jsonb NOT NULL DEFAULT '{}',  -- webhook payload, for audits
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_ref)             -- makes webhooks idempotent
);
CREATE INDEX purchases_user_idx ON purchases (user_id);

-- The single source of truth for access, whatever paid for it.
CREATE TABLE entitlements (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan         text NOT NULL,                 -- free | starter | pro | intl | mock_pack
    purchase_id  uuid REFERENCES purchases(id) ON DELETE SET NULL,
    status       text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'expired', 'revoked')),
    mocks_left   integer NOT NULL DEFAULT 0 CHECK (mocks_left >= 0),
    starts_at    timestamptz NOT NULL DEFAULT now(),
    ends_at      timestamptz,                   -- NULL = no end (one-time packs)
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX entitlements_user_active_idx ON entitlements (user_id) WHERE status = 'active';

-- Append-only minute wallet. Positive rows grant seconds, negative rows spend them.
CREATE TABLE minute_ledger (
    id           bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    delta_secs   integer NOT NULL CHECK (delta_secs <> 0),
    reason       text NOT NULL CHECK (reason IN
                 ('grant_free', 'grant_daily', 'purchase', 'subscription',
                  'usage', 'refund', 'adjustment')),
    session_id   uuid REFERENCES sessions(id) ON DELETE SET NULL,
    purchase_id  uuid REFERENCES purchases(id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CHECK (reason <> 'usage' OR (delta_secs < 0 AND session_id IS NOT NULL))
);
CREATE INDEX minute_ledger_user_idx ON minute_ledger (user_id);
-- A session can be debited only once, even if "complete" is retried.
CREATE UNIQUE INDEX minute_ledger_one_usage_per_session
    ON minute_ledger (session_id) WHERE reason = 'usage';
-- The one-time free grant can happen only once per user.
CREATE UNIQUE INDEX minute_ledger_one_free_grant
    ON minute_ledger (user_id) WHERE reason = 'grant_free';

CREATE VIEW minute_balances AS
    SELECT user_id, SUM(delta_secs)::bigint AS balance_secs
    FROM minute_ledger
    GROUP BY user_id;

-- ---------------------------------------------------------------- habit loop

CREATE TABLE streaks (
    user_id           uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    current_days      integer NOT NULL DEFAULT 0 CHECK (current_days >= 0),
    best_days         integer NOT NULL DEFAULT 0 CHECK (best_days >= 0),
    last_practice_on  date,
    updated_at        timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE push_tokens (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token         text NOT NULL UNIQUE,         -- Expo push token
    platform      text NOT NULL CHECK (platform IN ('android', 'ios', 'web')),
    created_at    timestamptz NOT NULL DEFAULT now(),
    last_seen_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX push_tokens_user_idx ON push_tokens (user_id);

-- ---------------------------------------------------------------- background jobs

-- A small Postgres-backed queue. Workers claim rows with
-- SELECT ... FOR UPDATE SKIP LOCKED, so several workers never take the same job.
CREATE TABLE jobs (
    id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind          text NOT NULL,                -- grade_session | send_reminder | ...
    payload       jsonb NOT NULL DEFAULT '{}',
    status        text NOT NULL DEFAULT 'queued'
                  CHECK (status IN ('queued', 'running', 'done', 'failed')),
    attempts      integer NOT NULL DEFAULT 0,
    max_attempts  integer NOT NULL DEFAULT 5,
    run_at        timestamptz NOT NULL DEFAULT now(),
    locked_at     timestamptz,
    last_error    text,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX jobs_ready_idx ON jobs (run_at) WHERE status = 'queued';
