# Voclara — development handoff

_Last updated: 23 Nov 2026. Written for continuing work in Cursor._

Voclara is an Urdu-first AI English speaking coach (IELTS-style mocks, job
interviews, daily talk) for South Asian learners. The plan, market notes and
the step-by-step prompts come from the product guide
(`Voclara — AI English Speaking Coach Product Building Guide.md`, kept outside
the repo). **Project rules for any AI assistant are in [`CLAUDE.md`](../CLAUDE.md)**,
and Cursor loads them through `.cursor/rules/voclara.mdc`.

---

## 1. Where things stand

| Guide step | What | Status |
| --- | --- | --- |
| Prompt 1 | Scaffold: Rust API, migrations, Docker, CI | ✅ Done, committed `ccfc48e` |
| Prompt 2 | Phone OTP + Google sign-in, rotating refresh tokens, minute wallet | ✅ Done, committed `1a2cb96` |
| Prompt 3 | `VoiceProvider` trait (Gemini, OpenAI, fake), `POST /sessions`, `/complete` | ✅ Done, committed `fd70f47` |
| Prompt 4 | Expo Android app + live audio | 🟡 Code done and tested; **native build not yet verified; not committed** |
| Prompt 4b | Next.js web layer | ⬜ Not started |
| Prompt 5 | Relay mode WebSocket | ✅ Done (API side; the mobile app still uses direct mode) |
| Prompt 6a | Grading from the transcript: jobs worker, grader LLM, feedback + progress, app screens | ✅ Done |
| UI redesign | "Indus & Marigold" design system (`apps/mobile/DESIGN.md`, `src/theme`), tabs, onboarding, animated splash, practice setup screens; API CORS for the web preview | ✅ Done |
| Prompt 6b | Pronunciation: clip upload (R2) + Azure Pronunciation Assessment | ⬜ Next |
| Prompt 7 | IELTS mock flow (Parts 1-3, result page, share card) | ⬜ Not started (examiner prompt exists) |
| Prompt 8 | Billing (Play Billing, Safepay, Paddle/Lemon Squeezy) | ⬜ Not started (tables exist) |
| Week 7 | Streaks, push, placement test | ⬜ Not started (tables exist) |
| Week 9 | Store listing, privacy policy, launch | ⬜ Not started |

**Tests:** API 130 passing (`cargo test` against Postgres), mobile 71 passing
(`npm test`). Clippy, `tsc`, `expo lint` and `expo-doctor` are all clean. The
relay load test ran 200 concurrent sessions × 10 round trips in 1.4 s
(`RELAY_LOAD_SESSIONS=200 cargo test --test relay many_concurrent`; default 50).

### Not verified yet (do these first)
1. **Android native build.** It has not succeeded yet. Kotlin and Java now
   compile with lean memory settings; the remaining blocker is Windows'
   260-character path limit in react-native-audio-api's C++ build. Fix it with
   option A (EAS cloud build) or B (long paths) in section 2. None of the
   failures so far were errors in our code.
2. **A real Gemini call.** The Gemini and OpenAI request shapes were written
   from the official docs and tested against a local mock, not the real services.
3. **Audio on a device:** mic capture, playback, echo cancellation in
   hands-free mode, and the session state machine with a real socket.
4. **Gemini session resumption** with a constrained (ephemeral) token. The app
   sends `setup.sessionResumption.handle` on reconnect, but the token locks the
   setup. It is unclear whether Gemini honours the handle; check on a 10+ minute
   mock.

---

## 2. Machine setup (this Windows PC)

| Thing | State / why |
| --- | --- |
| Rust | 1.100, **`stable-x86_64-pc-windows-gnu`** with MSYS2 GCC (`C:\msys64\mingw64\bin` on user PATH). The Visual Studio Build Tools installer hangs on this machine (its .NET downloader stalls on one manifest), so do not switch to the MSVC toolchain. |
| Postgres | Docker, `docker compose up -d db`, **port 5433** (5432 is used by another project's Postgres). |
| Node | 22.14, npm 11. |
| Android | SDK at `%LOCALAPPDATA%\Android\Sdk`, JDK 17, emulator AVD `Pixel_4_API_35`. |
| Android build | Needs **Git Bash before WSL bash** on PATH: `$env:Path = "C:\Program Files\Git\bin;$env:Path"`. |
| Android build memory | The first build failed with *"Could not reserve enough space in CodeHeap"*: the PC (16 GB) ran out of memory with the emulator, Docker, Gradle and Kotlin running together. Build with the emulator closed and lean flags: `.\gradlew.bat assembleDebug --no-parallel --max-workers=2 "-Dorg.gradle.jvmargs=-Xmx3g -XX:MaxMetaspaceSize=768m -XX:ReservedCodeCacheSize=256m" "-Pkotlin.compiler.execution.strategy=in-process"`. To make this permanent, put the same settings in `%USERPROFILE%\.gradle\gradle.properties`. |
| Android build path length | **Blocks local Android builds right now.** After the memory fix, the build failed with `ninja: error: mkdir(...): No such file or directory`. react-native-audio-api's C++ object folder is **269 characters**, over Windows' 260 limit, and `LongPathsEnabled` is 0 on this PC. What does not work: `subst` to a short drive (Expo autolinking resolves the real `E:\...` path, and at a drive root it cannot find `package.json` at all), or moving the repo to `C:\v` (saves only ~12 characters). See "Fixing the Android build" below. |

### Fixing the Android build (pick one)

**A. EAS cloud build (recommended; it is also what the guide plans for releases).**
It builds on Linux, so there is no path limit and it does not use this PC's RAM.
```powershell
cd apps/mobile
npx eas-cli@latest login                     # your Expo account
npx eas-cli@latest build:configure           # creates eas.json
npx eas-cli@latest build --profile development --platform android
```
Install the APK it produces on the emulator or phone, then run `npx expo start --dev-client`.

**B. Local build with Windows long paths.**
1. As administrator, enable long paths (this is a system setting):
   `New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name LongPathsEnabled -Value 1 -PropertyType DWORD -Force`, then reboot.
2. The ninja bundled with SDK CMake 3.22.1 may still fail on long paths. If it
   does, install a newer CMake in Android Studio's SDK Manager (SDK Tools ->
   CMake, 3.31+) so Gradle picks the newer ninja.
3. Delete the stale cache:
   `rd /s /q "\\?\E:\voclara\apps\mobile\node_modules\react-native-audio-api\android\.cxx"`.
4. With the emulator closed, build lean and emulator-only:
   `cd apps/mobile/android; .\gradlew.bat assembleDebug --no-parallel --max-workers=2 "-PreactNativeArchitectures=x86_64" "-Dorg.gradle.jvmargs=-Xmx3g -XX:MaxMetaspaceSize=768m -XX:ReservedCodeCacheSize=256m" "-Pkotlin.compiler.execution.strategy=in-process"`.
   Then start the emulator and `npx expo run:android`.

---

## 3. Commands

```powershell
# API (repo root)
docker compose up -d db
Copy-Item .env.example .env          # first time only
cargo run -p voclara-api             # http://127.0.0.1:8080
$env:DATABASE_URL="postgres://voclara:voclara@localhost:5433/voclara"; cargo test
cargo fmt --all; cargo clippy --all-targets -- -D warnings

# After changing the API: refresh the spec and TypeScript types (CI checks both)
cargo run -p voclara-api --bin export_openapi
cd apps/mobile; npm run gen:api

# Mobile (apps/mobile)
npm run typecheck; npm test; npx expo lint; npx expo-doctor
$env:Path = "C:\Program Files\Git\bin;$env:Path"
npx expo run:android                 # dev build on emulator/phone + Metro
```

For the emulator, run the API with `BIND_ADDR=0.0.0.0:8080`; the app's default
URL is `http://10.0.2.2:8080` (set `EXPO_PUBLIC_API_URL` in
`apps/mobile/.env.local` for a real phone). Sign-in codes are printed in the API
log (`OTP_PROVIDER=log`).

---

## 4. What exists

### Repository
```
apps/api/            Rust axum API (the control plane)
  migrations/        0001 core tables, 0002 otp_codes
  src/auth/          jwt.rs, refresh.rs (rotation + reuse detection), otp.rs, extractor.rs (AuthUser)
  src/domain/        minutes.rs (wallet), sessions.rs (lifecycle), users.rs
  src/providers/     otp.rs, google.rs, voice/{gemini,openai,fake}.rs
  src/routes/        health, auth, me, sessions
  src/bin/export_openapi.rs
  tests/             http, auth, minutes, sessions, voice_providers, migrations
apps/mobile/         Expo SDK 58 / RN 0.88 app (see apps/mobile/README.md)
  src/app/           sign-in, index (home), talk, feedback, progress, settings
  src/lib/api/       openapi-fetch client + single-flight TokenManager
  src/lib/live/      gemini.ts (protocol), pcm.ts, transcript.ts, session.ts (state machine)
  src/lib/audio/     mic.ts (16 kHz, AEC, M4A file), player.ts (24 kHz queue)
packages/prompts/    tutor-v1, examiner-v1, interviewer-v1, grader-v1 (embedded into the API)
packages/content/    ielts/cue-cards.json (2 sample cards)
packages/api-types/  openapi.json (exported) + schema.d.ts (generated)
.github/workflows/   api.yml, mobile.yml
```

### API endpoints (all JSON; errors are `{"error":{"code","message"}}`)
| Method | Path | Notes |
| --- | --- | --- |
| GET | `/health`, `/ready` | liveness / DB readiness |
| GET | `/openapi.json` | spec |
| POST | `/api/v1/auth/otp/send` | 6-digit code, 5 min, 30 s resend gap, 5/hour |
| POST | `/api/v1/auth/otp/verify` | sign in or sign up (+600 free seconds once) |
| POST | `/api/v1/auth/google` | ID token verified against Google JWKS; needs `GOOGLE_CLIENT_IDS` |
| POST | `/api/v1/auth/refresh` | rotates; reusing an old token revokes the family |
| POST | `/api/v1/auth/logout` | revokes the device's token family |
| GET/PATCH | `/api/v1/me` | profile, plan, balance, free drill secs today, streak; settings |
| POST | `/api/v1/sessions` | `{mode: talk\|mock\|interview, topic}` -> ephemeral voice token |
| POST | `/api/v1/sessions/{id}/complete` | typed transcript; charges time; logs cost (direct mode only; 409 for relay sessions) |
| GET | `/api/v1/sessions/{id}/ws?ticket=` | relay WebSocket (start the session with `relay: true`) |
| GET | `/api/v1/sessions/{id}/feedback` | grading status + bands, evidence, corrections, fillers, wpm |
| GET | `/api/v1/progress` | last 20 graded sessions, average of the last 5 real grades |

### Grading (step 6a)
- Completing a session (app `/complete` or relay close) queues one
  `grade_session` job in the same transaction and sets
  `sessions.grading_status = 'pending'`.
- Workers (`src/worker.rs`, `WORKER_CONCURRENCY` per process) claim jobs with
  `FOR UPDATE SKIP LOCKED`, retry with backoff (15 s, 30 s, 60 s… up to 1 h,
  5 attempts), and requeue jobs stuck `running` for 10+ minutes.
- The grader (`Grader` trait: Gemini `generateContent` with
  `responseJsonSchema`, OpenAI Chat Completions with strict `json_schema`, or
  `fake`) returns criterion bands, evidence quotes, corrections
  (`said, better, why_en, why_l1`) and a next focus. Prompt: `grader-v1`,
  with `{l1}` set to the learner's language name.
- **Rust decides the exact parts** (`domain/metrics.rs`): bands are clamped to
  0-9 and snapped *down* to 0.5 ("when unsure, pick the lower"); the overall
  band is the IELTS-style rounded mean (x.25 -> x.5, x.75 -> next whole);
  filler counts and words per minute are computed from the transcript.
  Pronunciation stays `null` until step 6b, even if the model returns one.
- Sessions with fewer than 25 learner words are `skipped`, not guessed.
- Scores from the fake grader get `grader_version = "fake"` and `demo: true`
  and are excluded from progress averages.
- Grading cost goes to `session_costs` (`provider = grader:<name>`, tokens in
  `tokens`, USD from `GRADER_USD_PER_1M_*`).
- Golden set: `packages/content/golden` (10 transcripts, **draft** target
  bands to be re-graded by a certified examiner).
  `GRADER_GOLDEN=1 cargo test -p voclara-api --test golden_grader -- --nocapture`
  runs the real grader and fails if any overall band is off by more than 0.5 or
  fewer than 80% of criterion bands are within 0.5.

### Relay mode (step 5)
`POST /api/v1/sessions {mode, topic, relay: true}` returns `transport: "relay"`,
`connect_url: /api/v1/sessions/{id}/ws` and a single-use `token` (the ticket,
valid 60 s). The app opens a WebSocket there with `?ticket=<token>`; the server
connects to the provider with **our** API key and sends **our** setup and
system prompt. Code: `src/relay.rs` (the select loop), `RelayCodec` per
provider in `src/providers/voice/{gemini,openai,fake}.rs`.

- App -> server: `{"type":"audio","seq":1,"pcm16_b64":"..."}` (16 kHz PCM16),
  `{"type":"end_turn"}`, `{"type":"stop"}`. Anything else ends the session
  (`protocol_error`), so the app cannot inject a setup.
- Server -> app: `ready`, `audio {seq, pcm16_b64, sample_rate}`,
  `caption {role, text, final}`, `interrupted`, `turn_complete`,
  `minutes {remaining_secs}` (every 10 s), `ended {reason}`.
- End reasons: `client_stopped`, `client_disconnected`, `minutes_exhausted`
  (hard cut-off at the limit), `idle` (60 s of silence from the app),
  `upstream_closed`, `provider_unavailable` (nothing charged), `provider_error`,
  `protocol_error`, `backpressure` (a peer blocked a send for 10 s).
- No internal queues: each send is awaited in the loop, so memory per session
  is bounded. App messages are capped at 256 KiB, audio chunks at 128 KiB.
- When the sockets close, the server completes the session itself: it charges
  the elapsed time (capped at the limit), stores the server-built transcript,
  and logs measured audio seconds in and out to `session_costs`.
- HTTP trace spans log the path only, never the query string (the ticket).
- Not verified against the real Gemini or OpenAI WebSocket services yet
  (tested with a fake upstream). The OpenAI codec resamples 16 -> 24 kHz and
  relies on server-side voice activity detection.

### Rules the code enforces (keep them)
- **Minutes:** integer seconds in an append-only `minute_ledger`. Every
  balance-then-write locks the user row first (`minutes::lock_user`), always in
  the order user, then session. One usage row per session (unique index).
- **Sessions:** one active session per user, with a limit of min(balance, mode
  limit: talk 600 s, mock/interview 900 s). Completion charges the
  *server-measured* elapsed time, capped at the limit. A session not completed
  2 minutes after its limit is charged in full, in **its own transaction**
  (`settle_abandoned_for`, also run on `GET /me`). A provider outage marks the
  session `failed` and charges nothing.
- **Voice:** Gemini tokens are single-use with the prompt locked in, and
  `expireTime` is a hard cut-off (`hard_cutoff: true`). An OpenAI client secret
  only limits session *start* (`hard_cutoff: false`); a real cut-off for OpenAI
  needs relay mode. Model ids come only from env.
- **Auth:** 15-minute HS256 access JWT; 30-day refresh tokens stored as SHA-256.
  The app's `TokenManager` refreshes single-flight; parallel refreshes would
  trigger reuse detection.
- **Prompts:** embedded via `include_str!`; their `version` is stored on each
  session. Never edit a published version; add `v2.md`.
- **Request bodies** use `AppJson<T>`, so malformed JSON gets our error format.
- Never log transcripts at info level; say "IELTS-style", never claim IELTS affiliation.

### Database tables
`users, refresh_tokens, otp_codes, sessions, session_costs, scores,
pronunciation_results, minute_ledger (+ minute_balances view), entitlements,
purchases, streaks, push_tokens, jobs` (the Postgres job queue that grading
runs on; see "Grading" above).

---

## 5. Decisions you still need to make
| Decision | Needed for |
| --- | --- |
| WhatsApp/SMS OTP provider (implement `OtpSender`) | any real sign-in; production refuses `OTP_PROVIDER=log` |
| Google OAuth client ids (Android + web) and a native sign-in library | Google sign-in button (API side is done) |
| Current Gemini Live model id + API key; measured cost per minute | real sessions; replace `VOICE_USD_PER_MIN=0.03` |
| Grader model id (`GRADER_PROVIDER`, `GRADER_MODEL`) and its token prices | real feedback (step 6a is built; it runs `fake` until set) |
| A certified IELTS examiner to re-grade `packages/content/golden` | trusting the golden test |
| Cloudflare R2 bucket and keys | clip upload (step 6b) |
| Azure Speech key and region | pronunciation scores (step 6b) |
| Payment providers and accounts | step 8 |

---

## 6. Next steps, in order (prompts to paste into Cursor)

Each prompt assumes Cursor has read `CLAUDE.md`. Finish each with the checks in
section 3 and a commit.

### 6.0 Finish step 4
1. Get a dev build installed (section 2, option A or B), then sign in on the
   emulator with the code from the API log.
2. Set `VOICE_PROVIDER=gemini`, `GEMINI_API_KEY`, `GEMINI_LIVE_MODEL` in `.env`
   and hold a real 2-minute conversation. Check the captions, the transcript
   stored in `sessions.transcript`, and the charge.
3. Test on a low-end Android phone over 4G (guide requirement).
4. Commit: "Add Expo Android app with live Gemini sessions".

> **Prompt (if the live audio misbehaves):** "Read apps/mobile/src/lib/live/session.ts
> and src/lib/audio/*. On a real device: [describe the symptom]. Check the
> react-native-audio-api 1.x docs for AudioRecorder/AudioBufferQueueSourceNode and
> fix it, keeping session.test.ts passing."

### 6.1 Prompt 4b — web layer
> "Create apps/web (Next.js App Router, TypeScript strict, Tailwind, shadcn/ui).
> Pages: landing (English + Urdu), a 6-question 'check your speaking band' quiz
> that records one 30-second answer in the browser and shows an estimated band
> (stub the scoring behind an interface until grading exists), email/phone
> capture, Play Store link, account page (plan, minutes from GET /api/v1/me),
> and an IELTS cue-card SEO page template generated from packages/content/ielts.
> Use the same API and bearer auth (phone OTP) via openapi-fetch and
> packages/api-types. Add CORS to the API for the web origin (tower-http
> CorsLayer, origins from config). Checkout comes in step 8; leave a placeholder."

### 6.2 Step 6b — pronunciation scoring (next backend step)
Step 6a grades from the transcript; this adds the pronunciation band.
Design notes: Azure's REST short-audio API does not accept the app's M4A, so
the app should upload short WAV clips (16 kHz PCM16 mono) of 3-5 learner
utterances that it already has as PCM while streaming. Check the current Azure
Pronunciation Assessment docs for unscripted (no reference text) assessment.
> "Implement step 6b. (1) An `AudioStore` trait with an R2 implementation
> that creates SigV4 presigned PUT/GET/DELETE URLs (R2_* env vars; prefer
> signing with the hmac/sha2 crates we already have over aws-sdk-s3, whose
> default TLS backend (aws-lc) does not build on this Windows GNU toolchain).
> POST /api/v1/sessions/{id}/clips/presign {count<=5} returns PUT URLs for
> `clips/{user}/{session}/{n}.wav`; POST /api/v1/sessions/{id}/clips/complete
> {clips:[{index, transcript_text}]} records them (audio_delete_at = now + 30
> days) and re-queues grading. (2) A `PronunciationScorer` trait plus Azure
> Pronunciation Assessment (AZURE_SPEECH_KEY/REGION, region in config) and a
> fake; store pronunciation_results (accuracy, fluency, prosody, completeness,
> words). (3) In domain/grading.rs pass a short summary of those scores to the
> grader (the `pronunciation_evidence` hook already exists) so the
> pronunciation band and its evidence are filled. (4) A daily job deletes
> clips past audio_delete_at. (5) In the app, keep the PCM of the longest 3-5
> learner turns during a session, write them as WAV, upload after /complete
> and call clips/complete. Fake impls and tests for every trait."

### 6.3 Prompt 7 — IELTS mock flow
> "Build the IELTS-style mock: Part 1 (4-5 short questions), Part 2 (cue card
> shown in the app, 60 s prep timer, 2 min talk, 1-2 rounding-off questions),
> Part 3 (4-5 discussion questions linked to the Part 2 topic), using
> packages/content/ielts (expand to 30+ cards). The examiner prompt is
> packages/prompts/examiner/v1.md; pass the chosen cue card to the session as
> its topic. Result page: overall band, four criterion bands with evidence, top
> 3 fixes, and a server-generated share card image with the Play Store link.
> Mocks consume entitlements.mocks_left when the plan includes them."

### 6.4 Relay mode in the app (optional)
The API side is done (section 4). To use it from the app, for example as the
path for the OpenAI fallback or for any provider when a hard server cut-off
matters:
> "In apps/mobile, add a relay transport to src/lib/live: when
> POST /api/v1/sessions is called with relay: true, open a WebSocket to
> API_URL + connect_url + '?ticket=' + token and speak the relay protocol in
> docs/HANDOFF.md (send 16 kHz PCM16 audio chunks, end_turn on push-to-talk
> release, stop on End; play `audio` at its sample_rate, show `caption`,
> clear playback on `interrupted`, show `minutes`). Do not call
> /sessions/{id}/complete for relay sessions; the server completes them.
> Reuse LiveSession's mic/player and keep session.test.ts passing; add tests
> with a fake socket."

### 6.5 Prompt 8 — billing
> "Add billing. App: Google Play Billing subscriptions and minute packs
> (RevenueCat optional), verified server-side. Web: Safepay (PKR) and Paddle or
> Lemon Squeezy (USD) webhooks at POST /billing/webhooks/{provider} with
> signature checks. Every source writes purchases (unique provider+provider_ref
> for idempotency) and updates entitlements + minute_ledger (minutes::grant) in
> one transaction. GET /api/v1/prices?country=PK from a prices table; never
> hard-code a price in the app. Show minutes in the app header and a top-up
> sheet at zero (replace the current 'coming soon' text on home and talk)."

### 6.6 Habit loop, then launch
- Streaks updated on session complete (in the user's timezone), push tokens
  (`POST /api/v1/push-tokens`, Expo Notifications), daily reminder job, WhatsApp
  utility template, and a placement test on first open (`POST /api/v1/placement`).
- Launch checklist: privacy policy (voice data, 30-day deletion, no training),
  delete-account endpoint (cascades already exist), Sentry, Play listing in
  English and Urdu, production config (`APP_ENV=production` refuses the dev
  secret, log OTP and fake voice).

---

## 7. Known gaps and small debts
- The mobile app has no Google sign-in button, no OpenAI WebRTC path
  (`transport: webrtc` shows "not supported") and no relay client yet.
- Relay sessions are tied to the instance that holds the socket. With several
  API instances behind a load balancer, WebSocket upgrades need sticky routing
  or a single relay instance.
- The local M4A recording is made but not uploaded (step 6).
- The feedback and progress screens show placeholders for bands.
- Home and talk show "plans coming soon" at zero minutes (step 8).
- `.env.example` still lists later-step variables with blank values.
- The 22 `npm audit` moderate warnings come from Expo/Jest dev dependencies;
  review them before release, and do not run `npm audit fix --force`, which
  breaks the SDK pins.
