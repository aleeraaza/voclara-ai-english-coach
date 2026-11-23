-- IELTS-style mock: which cue card (packages/content/ielts/cue-cards.json) a
-- mock session used, so results can show it and new mocks avoid repeats.
ALTER TABLE sessions ADD COLUMN cue_card_id text;

CREATE INDEX sessions_user_mock_recent_idx
    ON sessions (user_id, started_at DESC)
    WHERE mode = 'mock' AND cue_card_id IS NOT NULL;
