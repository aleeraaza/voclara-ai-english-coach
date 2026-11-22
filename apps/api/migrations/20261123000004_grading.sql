-- Grading: where each session's feedback stands, one grading job per session,
-- and which model produced a score.
ALTER TABLE sessions
    ADD COLUMN grading_status text NOT NULL DEFAULT 'none'
        CHECK (grading_status IN ('none', 'pending', 'ready', 'failed', 'skipped'));

-- A retried "complete" or a relay close must never queue grading twice.
CREATE UNIQUE INDEX jobs_one_grade_per_session
    ON jobs ((payload ->> 'session_id')) WHERE kind = 'grade_session';

-- Workers look for stuck jobs (crashed mid-run) by lock time.
CREATE INDEX jobs_running_locked_idx ON jobs (locked_at) WHERE status = 'running';

ALTER TABLE scores
    ADD COLUMN grader_model text,
    ADD COLUMN l1 text;          -- language the why_l1 explanations are written in
