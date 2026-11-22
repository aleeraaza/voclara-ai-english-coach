# Voclara

Voice-first AI English speaking coach for South Asian learners: live conversation,
IELTS-style speaking mocks, interview practice, pronunciation drills, with
corrections explained in Urdu (Hindi, Bengali, Arabic later).

## Repository

| Path | What | Status |
| --- | --- | --- |
| `apps/api` | Rust (axum) API: auth, sessions, minute wallet, grading, billing | Scaffolded |
| `apps/mobile` | Expo (React Native) Android app | Step 4 |
| `apps/web` | Next.js landing, quiz funnel, checkout, SEO | Step 4b |
| `packages/prompts` | Versioned tutor and grader prompts | v1 |
| `packages/content` | IELTS cue cards, interview banks, role-plays | Seeded |
| `packages/api-types` | TS types generated from `/openapi.json` | Step 4 |

## Run the API locally

Prerequisites: Rust stable (https://rustup.rs), Docker Desktop.

```bash
docker compose up -d db            # Postgres on localhost:5433
cp .env.example .env
cargo run -p voclara-api
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/openapi.json
```

Sign in locally: `POST /api/v1/auth/otp/send {"phone":"+923001234567"}`, read the
code from the server log (`OTP_PROVIDER=log` prints it), then
`POST /api/v1/auth/otp/verify {"phone":..., "code":...}` and call
`GET /api/v1/me` with `Authorization: Bearer <access_token>`.

Run database tests with `DATABASE_URL=postgres://voclara:voclara@localhost:5433/voclara cargo test`
(without it, database tests skip themselves).

Clients must never call `/auth/refresh` twice in parallel with the same token:
replaying a used refresh token is treated as theft and signs out that device.

### Live sessions

`VOICE_PROVIDER=fake` (the default) returns dummy tokens so the app can be built
without keys. For real voice set `VOICE_PROVIDER=gemini`, `GEMINI_API_KEY` and
`GEMINI_LIVE_MODEL` (optionally `VOICE_FALLBACK=openai` with the OpenAI keys).

- One running session per user; its limit is min(minutes left, mode limit:
  talk 10 min, mock/interview 15 min).
- Gemini tokens are single-use and Gemini rejects messages after the limit
  (`hard_cutoff: true`). OpenAI cannot be cut off server-side in direct mode
  (`hard_cutoff: false`): the app must end the call at `session_ends_at`.
- `complete` charges the time the server saw, capped at the limit. A session
  never completed is charged its full limit once it is 2 minutes overdue.
- Relay mode (`"relay": true` on session start): the app connects to
  `GET /api/v1/sessions/{id}/ws?ticket=…` and the server talks to the provider,
  cutting the session off at the limit for any provider. Protocol and end
  reasons are in [docs/HANDOFF.md](docs/HANDOFF.md).

Checks before every commit:

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
```

## Build plan

Following the product guide, one step at a time:

1. ✅ Scaffold: API skeleton, migrations, Docker, CI
2. ✅ Auth (phone OTP, Google, rotating refresh tokens), `/me`, minute wallet
3. ✅ VoiceProvider trait (Gemini Live, OpenAI Realtime, fake), `POST /api/v1/sessions`,
   `POST /api/v1/sessions/{id}/complete`
4. 🟡 Android app and live audio (code done; native build blocked by Windows
   path length, see docs/HANDOFF.md); 4b web layer
5. ✅ Relay mode (API side)
6. Grading pipeline ← next
7. IELTS mock flow
8. Billing
