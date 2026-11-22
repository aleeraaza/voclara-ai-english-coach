# Voclara mobile (Expo, Android first)

Expo SDK 58 · React Native 0.88 · Expo Router · TypeScript strict.

## Run on the Android emulator or a phone

The app uses native audio (`react-native-audio-api`), so it needs a
**development build**, not Expo Go.

```bash
# 1. API (repo root): Postgres + server, reachable from the emulator
docker compose up -d db
BIND_ADDR=0.0.0.0:8080 cargo run -p voclara-api

# 2. App
cd apps/mobile
cp .env.example .env.local     # set EXPO_PUBLIC_API_URL
npm install
npx expo run:android           # builds, installs and starts Metro
```

On Windows the native build needs a real `bash` before WSL's on PATH:
`$env:Path = "C:\Program Files\Git\bin;$env:Path"` (PowerShell).

Sign in with any phone number: with `OTP_PROVIDER=log` the API prints the
code in its log. With `VOICE_PROVIDER=fake` sessions start but cannot connect
(there is no real voice); set Gemini keys on the API for real conversations.

## Checks

```bash
npm run typecheck
npm test
npx expo lint
npx expo-doctor
```

## Layout

| Path | What |
| --- | --- |
| `src/app/` | Screens (Expo Router): sign-in, home, talk, feedback, progress, settings |
| `src/lib/api/` | Typed client (`openapi-fetch`) and single-flight token refresh |
| `src/lib/live/` | Gemini Live protocol, PCM/base64, transcript, session state machine |
| `src/lib/audio/` | Mic (16 kHz, echo-cancelled, also records M4A) and streamed playback |

API types come from `packages/api-types`. After changing the API:

```bash
cargo run -p voclara-api --bin export_openapi    # repo root
npm run gen:api                                  # apps/mobile
```

## How a live session works

1. `POST /api/v1/sessions` returns an ephemeral Gemini token, the WebSocket
   URL and `session_ends_at`.
2. The app connects, sends `setup`, waits for `setupComplete`, then streams
   mic audio (push-to-talk or hands-free) and plays the tutor's 24 kHz audio.
   Speaking over the tutor stops its audio at once.
3. The session ends when the learner taps End, at `session_ends_at`, when the
   app goes to the background, or if the connection is lost and cannot resume.
4. The transcript goes to `POST /api/v1/sessions/{id}/complete`, which charges
   the minutes. The local M4A recording is kept for upload once storage (R2)
   is set up.

Not yet in the app: Google sign-in (needs OAuth client ids), OpenAI WebRTC
fallback (`transport: webrtc`), audio upload, and graded feedback.
