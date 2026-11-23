# Voclara — project rules for AI assistants

## What this is
Urdu-first AI English speaking coach: live conversation, IELTS-style speaking
mocks, interview simulator, pronunciation drills. Android app first, small web
layer (landing, quiz funnel, checkout, account, SEO). One Rust API for both.

## Layout
- apps/api: Rust (axum) control plane — auth, sessions, minutes, grading jobs, billing
- apps/mobile: Expo SDK 58 (React Native 0.88) — the product. Routes in src/app,
  logic in src/lib. Read apps/mobile/AGENTS.md: always `npx expo install`, check
  versioned Expo docs, never edit the generated android/ ios/ folders.
- apps/web: Next.js — landing, quiz funnel, checkout, account, SEO (not built yet)
- packages/prompts: tutor personas and grader rubrics, versioned
- packages/content: IELTS cue cards, interview banks, role-plays
- packages/api-types: TypeScript types generated from the API's OpenAPI spec

## Stack
- apps/api: Rust stable, axum (ws), tokio, tokio-tungstenite, sqlx (Postgres),
  serde, reqwest, thiserror/anyhow, tracing, jsonwebtoken, aws-sdk-s3 (R2), utoipa
- apps/mobile: Expo (React Native), TypeScript strict, expo-router,
  react-native-webrtc (or provider RN SDK), expo-notifications, EAS Build
- apps/web: Next.js App Router, TypeScript strict, Tailwind, shadcn/ui
- packages/api-types: TypeScript types generated from the API's OpenAPI spec

## Rules
- Auth is bearer JWT (15-min access + rotating refresh token) for app AND web.
- Access is decided only by the entitlements table, whatever the payment source
  (Google Play, Safepay, Paddle/Lemon Squeezy).
- Voice providers, grader, pronunciation and TTS sit behind traits
  (VoiceProvider, Grader, PronunciationScorer, Tts) with fake impls for tests.
- Minutes are integers of seconds; debit atomically in SQL; no session may run
  past minutes_remaining.
- Log cost per session (provider, seconds in/out, tokens, USD) to session_costs.
- Never store raw audio longer than 30 days; never log transcripts at info level.
- Explanation language is a user setting (ur first; hi, bn, ar later). No hard-coded Urdu.
- No unwrap()/expect() outside tests. AppError -> IntoResponse.
- Prompts live in packages/prompts with a version string stored on every graded session.
- Model names come from config/env, never hard-coded.
- Say "IELTS-style practice"; never claim affiliation with IELTS.
- Run `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` before finishing.
- After any API change: `cargo run -p voclara-api --bin export_openapi`, then in
  apps/mobile `npm run gen:api`; CI fails if either file is stale.
- Mobile checks: `npm run typecheck && npm test && npx expo lint` in apps/mobile.
- Mobile UI follows apps/mobile/DESIGN.md: every colour, gradient, space, radius, font
  and duration comes from `useTheme()` (src/theme/tokens.ts is the only place raw values
  live); build screens from src/components; one marigold `cta` per screen; keep
  src/theme/theme.test.ts (AA contrast) passing when adding colour pairs.
- Request bodies use `AppJson<T>` (not `axum::Json`) so bad JSON gets our error format.
- Relay mode (src/relay.rs): the server always sends its own setup/prompt upstream;
  never forward a client setup. No unbounded queues: await sends in the select loop.
  Relay sessions are completed by the server, never by the app's /complete.
- Never log URLs with query strings (relay tickets travel in `?ticket=`).
- Grading: the LLM only judges; bands are snapped, the overall band, fillers and
  wpm are computed in domain/metrics.rs. Background work goes through the jobs
  table (src/worker.rs), never ad-hoc tokio::spawn from handlers.
- Tests in one file that drain the job queue must serialise (see tests/grading.rs).

## Local dev
- `docker compose up -d db` starts Postgres 16 on localhost:5433 (5432 is taken by other local Postgres).
- Windows toolchain: Rust stable-x86_64-pc-windows-gnu with MSYS2 MinGW GCC (C:\msys64\mingw64\bin on PATH).
  The MSVC Build Tools installer hangs on this machine, so do not switch to the msvc toolchain.
- Android native builds need Git Bash ahead of WSL's bash on PATH:
  `$env:Path = "C:\Program Files\Git\bin;$env:Path"`. Emulator AVD: Pixel_4_API_35.
- Copy `.env.example` to `.env`, then `cargo run -p voclara-api`.
- Migrations live in apps/api/migrations and run on startup when RUN_MIGRATIONS=true.

## I am learning Rust
Explain each borrow-checker or lifetime fix in 2-3 lines.
