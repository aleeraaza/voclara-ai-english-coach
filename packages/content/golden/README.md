# Golden grading set

Ten speaking transcripts with target bands, used by
`apps/api/tests/golden_grader.rs` to check the real grader against human
judgement (guide target: within 0.5 band on at least 80% of samples, and no
overall band off by more than 0.5).

**The current targets are DRAFT labels**, written and banded by the developer
to cover bands 4.5 to 8.0. Before trusting the test, have a certified IELTS
examiner re-grade each transcript and update `expected`. Real learner
transcripts (anonymised, with consent) are better than these written ones.

Run it (needs a real grader in `.env`):

```bash
# repo root, with GRADER_PROVIDER, GRADER_MODEL and the matching API key set
GRADER_GOLDEN=1 cargo test -p voclara-api --test golden_grader -- --nocapture
```

Each file: `id`, `labels`, `l1`, `expected` (fluency, lexical, grammar,
overall) and `turns` (`role`, `text`, `t_ms`), the same transcript shape the
app uploads. Pronunciation is not graded from text, so it is not in `expected`.
