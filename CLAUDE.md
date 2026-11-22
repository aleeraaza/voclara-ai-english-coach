# Voclara — project rules for AI assistants

## What this is
Urdu-first AI English speaking coach: live conversation, IELTS-style speaking
mocks, interview simulator, pronunciation drills. Android app first, small web
layer (landing, quiz funnel, checkout, account, SEO). One Rust API for both.

## Layout
- apps/api: Rust (axum) control plane — auth, sessions, minutes, grading jobs, billing
- apps/mobile: Expo (React Native) — the product (not built yet)
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

## Local dev
- `docker compose up -d db` starts Postgres 16 on localhost:5433 (5432 is taken by other local Postgres).
- Windows toolchain: Rust stable-x86_64-pc-windows-gnu with MSYS2 MinGW GCC (C:\msys64\mingw64\bin on PATH).
  The MSVC Build Tools installer hangs on this machine, so do not switch to the msvc toolchain.
- Copy `.env.example` to `.env`, then `cargo run -p voclara-api`.
- Migrations live in apps/api/migrations and run on startup when RUN_MIGRATIONS=true.

## I am learning Rust
Explain each borrow-checker or lifetime fix in 2-3 lines.
