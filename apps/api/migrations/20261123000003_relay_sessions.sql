-- Relay mode: the app connects to our WebSocket and we connect to the provider.
--   transport          'direct' (app talks to the provider) or 'relay' (through us)
--   relay_ticket_hash  SHA-256 of a single-use connect ticket; cleared on use
--   relay_prompt       the rendered system prompt, held until the relay opens
--                      (the server sends it upstream; the app never can)
ALTER TABLE sessions
    ADD COLUMN transport text NOT NULL DEFAULT 'direct' CHECK (transport IN ('direct', 'relay')),
    ADD COLUMN relay_ticket_hash bytea,
    ADD COLUMN relay_ticket_expires_at timestamptz,
    ADD COLUMN relay_prompt text;
