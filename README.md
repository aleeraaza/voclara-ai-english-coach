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

Checks before every commit:

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
```

## Build plan

Following the product guide, one step at a time:

1. ✅ Scaffold: API skeleton, migrations, Docker, CI
2. ✅ Auth (phone OTP, Google, rotating refresh tokens), `/me`, minute wallet
3. VoiceProvider trait and `POST /api/v1/sessions` ← next
4. Android app and live audio; 4b web layer
5. Relay mode (optional)
6. Grading pipeline
7. IELTS mock flow
8. Billing
