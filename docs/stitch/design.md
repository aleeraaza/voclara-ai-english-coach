# Voclara — complete app design brief (for Google Stitch)

> **How to use this file.** Give this whole document to Stitch as the design
> context. Then design one screen at a time using the **Stitch prompt** line at
> the end of each screen section (section 7). Every colour, font size, radius
> and spacing value below is the real value used in the app's code
> (`apps/mobile/src/theme/tokens.ts`), so designs can be built without
> translation. Screens marked **(planned)** are not built yet; design them in
> the same system.

---

## 1. Product in one page

**Voclara** is an AI English speaking coach for South Asian learners, Urdu
speakers first (Hindi, Bengali and Arabic later). Learners practise by
**talking out loud** to an AI voice, then get a band estimate and a few clear
fixes, with explanations in their own language.

| | |
| --- | --- |
| Platform | Android phone app first (portrait, 360–412 dp wide; design at **390 × 844**). A small web site comes later. |
| Audience | 17–35 year olds in Pakistan, India and Bangladesh: students preparing for IELTS, job seekers, people moving abroad, office workers. Many use mid-range Android phones on 4G. |
| Core promise | "Speak English with confidence: real conversations, an honest band, and fixes explained in Urdu." |
| Tone | Calm, warm, encouraging, never childish. A good teacher, not a game. |
| Legal wording | Always say **"IELTS-style practice"** and **"estimated band"**. Never imply an official IELTS score or any affiliation with IELTS. |

### Key features

1. **Talk with Sana (daily conversation).** A friendly 10-minute voice chat
   with the AI tutor "Sana" on any topic. Sana keeps the conversation going and
   fixes at most one mistake at a time. Hold-to-talk or hands-free.
2. **IELTS-style mock test.** A calm AI examiner runs all three parts in about
   15 minutes:
   - **Part 1:** 4–5 short questions.
   - **Part 2:** a cue card shown on screen, a 1-minute preparation timer,
     then up to 2 minutes of speaking.
   - **Part 3:** discussion questions on the same theme.

   The examiner never corrects you during the test. Afterwards you get the
   estimated overall band, the four criteria with evidence, the top 3 fixes and
   a shareable result card. There are 30+ cue card topics, and you get a new
   one each time.
3. **Job interview simulator.** Pick a role (software, sales, banking, nursing,
   call centre) and answer realistic interview questions.
4. **Feedback after every session.**
   - Estimated band (overall plus fluency, vocabulary, grammar and
     pronunciation) on the 0–9 IELTS-style scale, in 0.5 steps.
   - Quotes from what you said as evidence.
   - Corrections shown as "you said → better", with a short reason in English
     and an **"Explain in Urdu"** toggle.
   - Words per minute, filler words ("um", "like") and one "practise next"
     focus.
5. **Progress.** Band history, average of the last 5, graded-session count and
   streaks.
6. **Practice minutes.** Speaking time is metered in minutes. Free users get
   10 minutes, and plans and top-ups come later (paywall planned).
7. **Explanation language.** Urdu first. The user picks it in settings; the
   UI copy stays in English.
8. **Pronunciation drills (planned).** Short drills on sounds South Asian
   speakers find hard (v/w, th, short vowels), with per-word scores.

### Personas

- **Ayesha, 22, Lahore.** A final-year student targeting IELTS band 7 for a
  UK master's. Practises at night with earphones and wants mocks that feel
  real.
- **Bilal, 27, Karachi.** A call-centre agent aiming for a bank job. Short
  daily sessions on the bus, hold-to-talk. Wants interview practice.
- **Sana (the AI tutor).** Not a user. She appears as a glowing voice orb,
  never as a human photo or cartoon.

---

## 2. Brand: "Indus & Marigold"

- **Indus teal** (the river) is the brand colour: calm, trust, a clear voice.
- **Marigold** (genda, the garland flower) is the single warm accent. It is
  used for celebration, wins, streaks and **the one main call to action per
  screen**.
- **Ivory paper** is the light background; **deep ink-teal night** is the dark
  background. The live speaking screen is always dark, with a radial teal glow.
- **Visual signature:** the **voice orb**. It's a glowing sphere, a radial
  gradient from marigold through teal to deep teal. It breathes slowly when
  idle, shows a pulsing ring when listening, and glows marigold when the AI
  speaks. Sound-wave bars sit next to it.
- **Illustration style:** abstract and geometric (orbs, sound waves, soft
  gradient blobs, a band medallion). No stock photos and no cartoon mascots.
  Icons are Ionicons (rounded, filled for active and outline for inactive).

---

## 3. Design tokens

### 3.1 Colour palette (raw)

| Token | Hex | Token | Hex |
| --- | --- | --- | --- |
| indus50 | `#EAF6F4` | marigold100 | `#FDEDD3` |
| indus100 | `#D3EEEA` | marigold300 | `#F8C878` |
| indus200 | `#A6DCD5` | marigold400 | `#F5B14A` |
| indus300 | `#6FC4BA` | marigold500 | `#EE9B1F` |
| indus400 | `#35A89C` | marigold700 | `#9A5B05` |
| indus500 | `#0E8C80` | coral100 / 300 / 500 | `#FCE2DF` / `#F08A82` / `#E2554C` |
| indus600 | `#0B7369` | leaf100 / 300 / 500 | `#DCF3E6` / `#5CC791` / `#23935E` |
| indus700 | `#095B53` | iris100 / 300 / 500 | `#E6E7FB` / `#8F96EC` / `#5560D6` |
| indus800 | `#073F3A` | ivory | `#F7F5EF` |
| indus900 | `#05302C` | paper | `#FFFFFF` |
| indus950 | `#031E1C` | sand / stone | `#ECE9E0` / `#DCD8CC` |
| night | `#071413` | ink / slate | `#0E1E1C` / `#55655F` |
| nightRaised | `#0E201E` | mist / fog | `#E8F1EF` / `#94AAA5` |
| nightHigh | `#152D2A` | nightLine | `#22403B` |

### 3.2 Colour roles (use these in designs)

| Role | Light | Dark | Used for |
| --- | --- | --- | --- |
| bg | `#F7F5EF` ivory | `#071413` night | Screen background |
| surface | `#FFFFFF` | `#0E201E` | Cards, sheets, inputs, tab bar |
| surfaceAlt | `#ECE9E0` | `#152D2A` | Chips, progress tracks, pressed rows |
| border | `#DCD8CC` | `#22403B` | Hairlines, input borders |
| text | `#0E1E1C` | `#E8F1EF` | Body and headings |
| textMuted | `#55655F` | `#94AAA5` | Secondary text |
| primary | `#0B7369` | `#6FC4BA` | Brand buttons, links, selected states |
| onPrimary | `#FFFFFF` | `#031E1C` | Text on primary |
| primarySoft | `#EAF6F4` | `#123A36` | Tinted icon backgrounds, selected chips |
| accent | `#EE9B1F` | `#F5B14A` | The single CTA glow, wins, streak flame |
| onAccent | `#2A1900` | `#2A1900` | Text on marigold |
| accentSoft | `#FDEDD3` | `#3A2A10` | Soft marigold backgrounds |
| success / successSoft | `#23935E` / `#DCF3E6` | `#5CC791` / `#133B29` | "Better" corrections, done |
| danger / dangerSoft | `#E2554C` / `#FCE2DF` | `#F08A82` / `#3D1C1A` | Errors, "you said" strikethrough, sign out |
| info / infoSoft | `#5560D6` / `#E6E7FB` | `#8F96EC` / `#20244A` | Interview mode, charts, info notices |
| onCanvas / onCanvasMuted | `#E8F1EF` / `#94AAA5` | same | Text on the dark live canvas |
| onBrand / onBrandMuted | `#FFFFFF` / white 84% | same | Text on teal gradients |
| glass / glassStrong | white 10% / white 18% | same | Buttons and bubbles on the canvas |
| learnerBubble | `rgba(111,196,186,0.22)` | same | The learner's caption bubble |
| overlay | `rgba(7,20,19,0.55)` | `rgba(0,0,0,0.6)` | Modal scrim |

All text pairs meet **WCAG AA** (4.5:1 for body text, 3:1 for large text).

### 3.3 Gradients

| Name | Light | Dark | Use |
| --- | --- | --- | --- |
| brand | 135°: indus800 → indus600 (55%) → indus400 | 135°: indus900 → indus700 → indus500 | Hero cards, splash, profile header |
| canvas | radial at 50% 35%: indus800 → night (70%) | same | The live session screen (always dark) |
| sunrise | 135°: marigold400 → coral500 | same | The main CTA button, held talk button |
| wash | 180°: indus100 → ivory | 180°: nightHigh → night | Soft tint behind the top ~260 px of tab screens |
| orb | radial at 35% 30%: marigold300 → indus400 (45%) → indus700 | … → indus800 | Voice orb |
| talk | 135°: indus500 → indus700 | indus400 → indus700 | "Talk" mode tile |
| mock | 135°: marigold400 → marigold500 | same | "Mock test" mode tile and header |
| interview | 135°: iris300 → iris500 | same | "Interview" mode tile |
| drill | 135°: coral300 → coral500 | same | "Pronunciation" mode tile |

### 3.4 Typography: Plus Jakarta Sans

| Style | Weight | Size / line height | Letter spacing | Use |
| --- | --- | --- | --- | --- |
| display | ExtraBold 800 | 34 / 40 | -0.6 | Splash wordmark, onboarding titles |
| title | Bold 700 | 24 / 30 | -0.3 | Screen titles, hero headings |
| heading | Bold 700 | 18 / 24 | -0.1 | Card titles |
| body | Medium 500 | 16 / 24 | 0 | Body copy |
| bodyStrong | Bold 700 | 16 / 24 | 0 | Emphasis, button labels |
| small | Medium 500 | 14 / 20 | 0 | Secondary copy |
| caption | Medium 500 | 12 / 16 | 0 | Meta text, hints |
| label | Bold 700 | 12 / 16, UPPERCASE | +0.8 | Section labels, eyebrows |
| metric | ExtraBold 800 | 44 / 48, tabular numbers | -1 | Bands, timers, minutes |

Urdu explanations use the system Nastaliq/Naskh font at **18 / 30** with
right-to-left direction, in a marigold-soft box.

### 3.5 Spacing, radius, elevation, motion

- **Spacing (4-pt grid):** 2, 4, 8, 12, 16, 20, 24, 32, 48. Screen padding is
  16, gaps between cards are 16, and card padding is 16.
- **Radius:** sm 10 · md 14 · lg 20 (cards) · xl 28 (hero cards) · pill 999
  (buttons, chips).
- **Elevation:**
  - Light theme:
    - card: `0 1 2 rgba(14,30,28,.06), 0 4 14 rgba(14,30,28,.06)`
    - raised: `0 8 24 rgba(14,30,28,.14)`
    - glow (CTA): `0 10 30 #EE9B1F66`
  - Dark theme: cards are flat (1 px top highlight), and raised is
    `0 10 28 rgba(0,0,0,.45)`.
- **Motion:** fast 150 ms (press), base 250 ms, slow 400 ms, intro 700 ms,
  orb breathing 1400 ms. Press feedback scales to 0.97. Nothing flashes or
  loops faster than 1.4 s.
- **Touch targets:** at least 48 × 48 dp. Primary buttons are 52 dp tall
  (58 dp for "lg").

### 3.6 Tokens as CSS variables (for Stitch's HTML export)

```css
:root { /* light */
  --bg:#F7F5EF; --surface:#FFFFFF; --surface-alt:#ECE9E0; --border:#DCD8CC;
  --text:#0E1E1C; --text-muted:#55655F;
  --primary:#0B7369; --on-primary:#FFFFFF; --primary-soft:#EAF6F4;
  --accent:#EE9B1F; --on-accent:#2A1900; --accent-soft:#FDEDD3;
  --success:#23935E; --danger:#E2554C; --info:#5560D6;
  --grad-brand:linear-gradient(135deg,#073F3A 0%,#0B7369 55%,#35A89C 100%);
  --grad-sunrise:linear-gradient(135deg,#F5B14A 0%,#E2554C 100%);
  --grad-canvas:radial-gradient(circle at 50% 35%,#073F3A 0%,#071413 70%);
  --grad-orb:radial-gradient(circle at 35% 30%,#F8C878 0%,#35A89C 45%,#095B53 100%);
  --radius-card:20px; --radius-hero:28px; --radius-pill:999px;
  --font:'Plus Jakarta Sans',system-ui,sans-serif;
}
[data-theme="dark"] {
  --bg:#071413; --surface:#0E201E; --surface-alt:#152D2A; --border:#22403B;
  --text:#E8F1EF; --text-muted:#94AAA5;
  --primary:#6FC4BA; --on-primary:#031E1C; --primary-soft:#123A36;
  --accent:#F5B14A; --accent-soft:#3A2A10;
  --success:#5CC791; --danger:#F08A82; --info:#8F96EC;
}
```

---

## 4. Components

| Component | Anatomy and variants | Rules |
| --- | --- | --- |
| **Screen** | Safe area, bg colour, optional `wash` gradient at the top, large title (24) with a muted subtitle and an optional trailing avatar | Scrolls; 16 padding; 48 bottom padding |
| **Button** | Pill, 52 dp (lg 58 dp), icon left + bodyStrong label. Variants: **primary** (teal fill), **cta** (sunrise gradient, dark text, marigold glow), **secondary** (surface fill + border), **ghost** (text only, teal), **danger** (coral) | **At most one `cta` per screen.** Disabled = 50% opacity. Loading = spinner in place of the label. |
| **IconButton** | 48 dp circle; tones: surface (card shadow), ghost, canvas (glass on dark) | Always has a screen-reader label |
| **Card** | Radius 20, padding 16, gap 12. Tones: default (surface + shadow), soft (primarySoft, no shadow), outline (border only). Can be tappable | Tappable cards shrink to 0.97 when pressed |
| **ModeTile** | Gradient square card (mode gradient), white icon in a glass circle, title + subtitle, optional badge ("10 min", "Soon"); a compact version for two-up rows | One gradient per mode, never mixed |
| **StatWidget** | Small card: tinted icon circle, metric number, caption label | Usually three in a row on Home |
| **BandBadge** | Circular marigold medallion (sunrise gradient + inner ring), band number in metric style, label below | Hero of every result |
| **ProgressBar** | 8 dp track (surfaceAlt), teal fill, animated | Criteria bands use value/9 |
| **Chip / ChipGroup** | Pill, 40 dp, icon + label; selected = primarySoft fill + primary text + border | Topic picker, talk-mode picker, level picker |
| **ListRow / ListGroup** | 56 dp row: tinted icon circle, title, subtitle, value, chevron. The group is one card with dividers | Settings, history |
| **Field** | Label above, 52 dp input with leading icon, radius 14, hint or error below | Focus = primary border |
| **Notice** | Rounded banner with icon. Tones: danger, info, success | Inline errors and tips |
| **EmptyState** | Large tinted icon circle, title, body, optional action button | Every empty list |
| **VoiceOrb** | Gradient sphere (orb gradient) with a soft glow. States: idle (slow breathing), listening (teal pulsing ring), speaking (marigold ring + glow). Sizes 84–150 | Only on speaking-related screens |
| **SoundWave** | 4–12 rounded bars that animate in height | Next to status text and on the splash |
| **Caption bubble** | On the canvas: tutor = glass, left-aligned; learner = learnerBubble, right-aligned; 85% max width, radius 20 with one 4 px corner; small "Sana"/"You" label | Shows the last 8 turns |
| **MockStage** | On the canvas: a 3-step pill stepper (Part 1 · 2 · 3; current = glassStrong with marigold border and dot, done = marigold check). In Part 2 it adds a **cue card** panel (glass): "CUE CARD · Travel" label, the task in bold, "You should say:" bullets, and a timer pill (marigold "Prepare 0:42 / 1:00", then glass "Speak 1:37 / 2:00") | See screen 7.9 |
| **Tab bar** | 4 tabs: Home, Practice, Progress, Profile. Surface background, raised shadow, 68 dp. The active icon sits in a 52 × 30 primarySoft pill, filled icon + teal label | Hidden on the live screen |

---

## 5. Navigation map

```
Splash (animated)
 ├─ first launch → Onboarding (3 slides) → Sign in (phone → code)
 └─ signed in → Tabs
     ├─ Home ──────────┬─ Talk with Sana (hero CTA) → Live session → Session result
     │                 ├─ Mock test tile → Mock setup → Live mock → Mock result
     │                 └─ Interview tile → Interview setup → Live interview → Result
     ├─ Practice ──────┬─ Talk setup (topic) ┐
     │                 ├─ Mock setup         ├→ Live session → Result
     │                 ├─ Interview setup    ┘
     │                 └─ Pronunciation drills (planned)
     ├─ Progress ──────── History row → Past session result
     └─ Profile ───────┬─ Explanation language
                       ├─ Level & goal
                       ├─ Plans & minutes (planned paywall)
                       └─ About & privacy · Sign out
```

Rules:

- **One job per screen.** Setup screens choose; the live screen speaks;
  result screens explain.
- The back arrow sits top left on pushed screens. The live screen has a ✕
  ("End session") in the top left instead.
- Leaving the live screen always saves the session and opens its result.

---

## 6. Global UX rules

- **One marigold CTA per screen.** Everything else is primary, secondary or
  ghost.
- **Thumb zone.** Primary actions sit at the bottom of setup screens and the
  live screen.
- **Speaking first.** The talk button is 104 dp and centred; its label is
  "Hold to talk". Hands-free mode replaces it with "End session".
- **Kind feedback.**
  - Show the band first, then what's good (evidence), then at most 3–6 fixes
    (exactly the top 3 for mocks).
  - The "you said" text is coral with a strikethrough; the "better" text is
    green and bold.
- **Every state is designed:** loading (skeleton or spinner plus a reassuring
  line), empty (EmptyState with an action), error (Notice plus Retry),
  offline, out of minutes.
- **Copy:** short sentences, second person, British spelling ("practise",
  "colour"). Never "fail" or "wrong". Use "try this", "better", "fix".
- **Accessibility:** AA contrast, 48 dp targets, screen-reader labels on all
  icon buttons, captions for everything spoken, timers announced politely.
- **Dark mode:** follows the system setting. The live screen is always dark.

---

## 7. Screens

Each screen lists its purpose, layout from top to bottom, real copy, states,
and a prompt for Stitch.

### 7.1 Splash (animated loading)

- **Purpose:** brand moment while fonts and the session load. Shown for at
  least 1.4 s, then fades into the app.
- **Layout:**
  - Full-bleed **brand** gradient.
  - The voice orb (≈120 dp) springs in at the centre.
  - SoundWave bars below it.
  - The "Voclara" wordmark (display 34, white) and the tagline "Speak English
    with confidence" (onBrandMuted) rise into place.
- **Motion:** orb spring 0 → 1 (700 ms), wordmark fades up 12 px, tagline
  follows 150 ms later, the whole layer fades out over 400 ms.
- **Stitch prompt:** "Android splash screen, full-bleed deep-teal diagonal
  gradient (#073F3A→#0B7369→#35A89C), a glowing radial orb (marigold #F8C878
  core into teal) centred, small animated sound-wave bars under it, wordmark
  'Voclara' in Plus Jakarta Sans ExtraBold 34 white, tagline 'Speak English
  with confidence' in white 84%."

### 7.2 Onboarding (3 slides)

- **Purpose:** explain the value in 15 seconds; shown once, before sign-in.
- **Layout:**
  - Top right: "Skip" (ghost).
  - The illustration takes the top ~55%.
  - Title (display 28–34) and body (body, muted).
  - Page dots (active dot = 24 × 8 pill in primary).
  - Bottom: a cta button, "Next" on slides 1–2 and "Get started" on slide 3.
- **Slides:**
  1. **"Talk, don't type."** "Have real spoken conversations with Sana, your
     AI English coach, any time." Art: the speaking orb with two caption
     bubbles ("Hi! I'm Sana. Tell me about your day 🙂" / "I went to
     university and…").
  2. **"Know your band."** "IELTS-style mocks give you an estimated band and
     the 3 fixes that matter most." Art: the BandBadge medallion showing 6.5
     with four small criteria bars.
  3. **"Explained in Urdu."** "Every correction comes with a simple
     explanation in your language." Art: a correction card with an Urdu
     explanation box: "“کل” ماضی ہے، اس لیے go کی جگہ went بولیں۔".
- **Stitch prompt:** "Onboarding slide on ivory #F7F5EF with a soft teal wash
  at the top, large abstract illustration (glowing orb + chat bubbles), title
  'Talk, don't type.' Plus Jakarta Sans ExtraBold, muted body, pill page dots,
  full-width marigold-to-coral gradient pill button 'Next', 'Skip' text button
  top right."

### 7.3 Sign in (phone number → code)

- **Purpose:** passwordless sign-in by mobile number (Google sign-in comes
  later).
- **Layout:**
  - Wash gradient at the top.
  - Orb (96).
  - Title "Welcome to Voclara" and subtitle "Sign in with your mobile number.
    No password needed."
  - Field "Mobile number" (phone icon, placeholder "0300 1234567", hint
    "Pakistan numbers work as 03xx. Abroad? Start with +country code.").
  - A cta "Send code" (lg, paper-plane icon).
  - Legal caption at the bottom: "IELTS-style practice. Voclara is not
    affiliated with IELTS. By continuing you agree to our terms."
- **Code step:**
  - Title "Enter your code" and "We sent a 6-digit code to +92300…".
  - A 6-digit field with wide letter spacing.
  - A cta "Sign in", plus the ghost "Use a different number".
- **States:** sending (button spinner), wrong code (danger Notice "That code
  is not right. Try again."), offline ("Cannot reach Voclara. Check your
  internet.").
- **Stitch prompt:** "Mobile sign-in, ivory background with teal wash,
  glowing orb, title 'Welcome to Voclara', single phone input with phone icon
  and helper text, full-width sunrise-gradient pill button 'Send code', tiny
  legal caption at the bottom."

### 7.4 Home (tab)

- **Purpose:** start today's practice in one tap and see your status.
- **Layout:**
  1. **Header:** "Good morning" (small, muted) and "Ready to speak? 👋"
     (title). The right side has a 44 dp avatar circle with an initial on the
     brand gradient, which opens Profile.
  2. **Hero card** (brand gradient, radius 28):
     - Eyebrow "DAILY CONVERSATION" (label, onBrandMuted).
     - Title "Talk with Sana".
     - Body "A friendly 10-minute chat on any topic. She fixes one mistake at
       a time."
     - A cta "Start talking" (mic icon).
     - A small orb (70) top right.
  3. **Three StatWidgets:** "10 min · Practice left" (clock, teal), "0 · Day
     streak" (flame, marigold) and "– · Latest band" (ribbon, iris).
  4. **Section "PRACTISE FOR A GOAL"** with a "See all" link, then two compact
     ModeTiles: **Mock test** "IELTS-style, Parts 1–3" (mock gradient) and
     **Interview** "Pick a role, get grilled" (interview gradient).
  5. **Tip of the day card** (primarySoft): a bulb icon, "TIP OF THE DAY" and
     "Record a 2-minute talk daily. Small, regular practice beats one long
     session."
- **States:**
  - Out of minutes: the hero CTA turns into "See plans" (planned) and a
    Notice appears.
  - First day: streak 0 with the copy "Start your streak today".
- **Stitch prompt:** "Android home screen for an English speaking coach.
  Ivory background, greeting header with avatar, a large deep-teal gradient
  hero card 'Talk with Sana' with a glowing orb and a marigold-coral gradient
  pill button 'Start talking', a row of three small stat cards (minutes,
  streak flame, latest band), two square gradient mode tiles (amber 'Mock
  test', indigo 'Interview'), a soft-teal tip card, bottom tab bar with 4
  tabs (Home active)."

### 7.5 Practice (tab)

- **Purpose:** choose a practice mode.
- **Layout:**
  - Title "Practice" and subtitle "Choose how you want to speak today."
  - Four full-width ModeTiles, each with an icon square, title, subtitle and
    a time badge:
    - **Talk with Sana:** "Relaxed conversation on a topic you choose" · 10 min.
    - **IELTS-style mock test:** "Parts 1, 2 and 3 with an examiner, then
      your band" · 15 min.
    - **Job interview:** "Software, sales, banking, nursing or call centre" ·
      15 min.
    - **Pronunciation drills:** "v / w, th, short vowels and more" · "Soon"
      (disabled, 50%).
  - A **"HOW IT WORKS"** card with three numbered rows: "1. Pick a mode",
    "2. Speak naturally" and "3. Get your band and fixes".
- **Stitch prompt:** "Practice menu: four stacked cards, each with a coloured
  gradient icon square (teal, amber, indigo, coral), title, two-line
  subtitle and a small pill time badge; the last card is dimmed with a 'Soon'
  badge; a 'How it works' card with 3 numbered steps below."

### 7.6 Talk setup

- **Purpose:** pick a topic, then start.
- **Layout:**
  - Header card (talk gradient): "Talk with Sana", "Choose a topic or let
    Sana pick."
  - Section "PICK A TOPIC" with a ChipGroup (each chip has an icon): My day,
    Work & study, Hometown, Cricket, Food, Travel, Technology, Future plans.
  - Field "Or type your own topic" (max 120 characters).
  - **StartPanel card:**
    - "HOW YOU'LL TALK" with chips "Hold to talk" / "Hands-free" and a hint
      line.
    - "Up to 10 min this session · 10 min left".
    - A cta "Start session" (lg, mic icon).
- **Stitch prompt:** "Setup screen: teal gradient header card, a wrap of
  selectable topic chips (one selected in soft teal), an optional text input,
  and a bottom card with a two-option segmented chip control ('Hold to talk' /
  'Hands-free'), minutes line, and a full-width sunrise gradient 'Start
  session' button."

### 7.7 Mock setup

- **Purpose:** prepare the learner for the IELTS-style mock.
- **Layout:**
  - Header (mock marigold gradient, dark text): "ABOUT 15 MINUTES" and
    "IELTS-style speaking test". Body: "A calm examiner runs all three parts.
    Your estimated band arrives right after."
  - Section "WHAT TO EXPECT": a vertical timeline card with three steps
    joined by a line. Each has a marigold-soft icon circle, title, duration
    and body:
    - **Part 1 · Introduction** (4–5 min): "Short questions about you, your
      home, work or studies."
    - **Part 2 · Long turn** (3–4 min): "A cue card on your screen: one
      minute to prepare, then speak for up to two minutes."
    - **Part 3 · Discussion** (4–5 min): "Deeper questions linked to your
      Part 2 topic."
  - An info Notice: "Find a quiet place. Each test uses a new topic from 30+
    cue cards. The examiner does not correct you during the test; your band
    and top 3 fixes come after."
  - The StartPanel (mock, 15 min).
- **Planned:** a "Mocks left: 2" pill on the header for paid mock packs.
- **Stitch prompt:** "Mock test intro: amber gradient header card 'IELTS-style
  speaking test', a vertical 3-step timeline (Part 1, Part 2, Part 3) with
  icon circles and durations, an indigo info banner, and a start panel with a
  talk-mode toggle and a full-width sunrise gradient 'Start session' button."

### 7.8 Live session (Talk / Interview): always dark

- **Purpose:** speak. Nothing else.
- **Layout on the canvas gradient:**
  - **Top bar:** ✕ (glass IconButton, "End session"); centre: mode label
    (label, muted) and the topic (small); right: a timer pill "09:42" (glass,
    which turns marigold with dark text in the last minute).
  - **Orb** (150) centred, with SoundWave and status text below: "Connecting
    to Sana…", "Sana is speaking", "Hold the button and speak", "Listening…",
    "Reconnecting…", "Saving your session…".
  - **Captions:** the last 8 turns as bubbles (Sana left in glass, You right
    in teal). Empty hint: "Captions appear here as you talk."
  - **Controls (bottom):**
    - A segmented pill: "Hold to talk" | "Hands-free".
    - A **104 dp talk button**: talk gradient with a mic-outline icon; while
      held it switches to the sunrise gradient, a filled mic, a glow and an
      expanding marigold ripple ring.
    - The caption "Tap ✕ to finish and get your feedback".
    - Hands-free mode shows a secondary "End session" button instead.
- **Blocked states** (same canvas, centred):
  - Microphone denied: mic-off icon, "Microphone needed", cta "Open
    settings".
  - Out of minutes: hourglass icon, "Out of minutes", "You have used your
    practice minutes. Plans and top-ups are coming soon."
  - Couldn't start: cloud-offline icon plus the reason, with "Go back".
- **Stitch prompt:** "Immersive dark voice-chat screen: radial deep-teal glow
  on near-black (#073F3A→#071413), top bar with a glass close button, mode
  label and a pill timer '09:42', a large glowing orb in the upper third with
  a marigold ring (AI speaking), status 'Sana is speaking' with tiny sound
  bars, chat-caption bubbles (left translucent white, right translucent
  teal), bottom segmented toggle 'Hold to talk | Hands-free' and a big round
  104 dp mic button with a marigold ripple."

### 7.9 Live mock test (new): always dark

- **Purpose:** run the IELTS-style mock, showing where you are and the cue
  card.
- **Layout:** the same canvas and top bar (centre: "IELTS-STYLE MOCK" and the
  theme, e.g. "Travel").
  - Orb **110** (84 during Part 2).
  - **MockStage** under the orb:
    - **Stepper:** three equal pills: "✓ Part 1", "● Part 2", "Part 3".
    - **Part 2 cue card panel** (glass, radius 20, padding 16):
      - Row: "CUE CARD · TRAVEL" (label, muted) and a **timer pill** on the
        right.
      - **"Describe a memorable journey you have made."** (bodyStrong).
      - "You should say:" with four bullets: "where you went", "how you
        travelled", "who you went with", "and explain why it was memorable".
      - A hint line that changes with the stage: "Plan your answer. Think of
        one example for each point." → "Speak now. Keep going until the
        examiner stops you." → "Time is up. Finish your sentence."
    - **Timer pill:** preparing = marigold fill with an hourglass, "Prepare
      0:42 / 1:00"; speaking = glassStrong with a mic, "Speak 1:37 / 2:00";
      done = "Time".
    - **Part 3:** one muted line, "💬 Discussion linked to: Describe a
      memorable journey…".
  - Captions are labelled "Examiner" and "You". Status: "The examiner is
    speaking".
  - **End of test:** when the examiner says "That is the end of the speaking
    test", a cta **"Get my band"** (ribbon icon, lg, full width) appears above
    the talk controls.
- **Stitch prompt:** "Dark immersive IELTS-style mock screen: smaller glowing
  orb at the top, a 3-step pill stepper (Part 1 done with marigold check,
  Part 2 current with marigold outline), a translucent glass cue-card panel
  with label 'CUE CARD · TRAVEL', bold task 'Describe a memorable journey you
  have made.', four bullet points, and a marigold timer pill 'Prepare 0:42 /
  1:00'; caption bubbles below; big round mic button at the bottom."

### 7.10 Session result (after Talk / Interview)

- **Purpose:** celebrate effort, show the band, give the fixes.
- **Layout:**
  1. Two summary cards side by side: "✓ Session finished · 8 min used" and
     "🎙 412 words · 2 min left".
  2. **Feedback:**
     - **Pending:** a card with a spinner, "Sana is reviewing your answers…"
       and "Usually under a minute. You can stay here or come back from
       Progress."
     - **Ready:**
       - Hero card: an optional "DEMO FEEDBACK" pill, a BandBadge (132) with
         6.5, a headline by band ("Good progress!", "Excellent work!", "Nice
         effort, keep going!"), "Estimated overall band · IELTS-style
         practice" and a secondary "Share result" button.
       - **YOUR FOUR CRITERIA** card: four rows (icon, name, value, animated
         bar and a quoted evidence line). Pronunciation shows "–" and
         "Pronunciation scoring is coming soon."
       - **"5 fixes to try":** correction cards with a coral strikethrough
         "I go to office yesterday", a green bold "I went to the office
         yesterday", the reason "Use the past tense for finished actions." and
         a teal toggle "Explain in Urdu". Expanded, it shows an RTL Urdu text
         box on marigold-soft.
       - **HOW YOU SPOKE:** two stat cards, "112 · Words per minute" and "7 ·
         Fillers: “um”, “like”".
       - A **"Practise next"** soft card with a flag icon and the focus
         ("Use past tense for yesterday's events").
     - **Skipped** (too short): an EmptyState "A little more speaking next
       time" with "We need at least 40 words from you…".
     - **Failed / offline:** an EmptyState "Feedback unavailable" with a
       "Try again" secondary button.
  3. **WHAT YOU SAID:** a card of bullet lines from the learner's turns.
  4. A cta "Practise again" and a ghost "Back to home".
- **Stitch prompt:** "Result screen on ivory: a hero card with a large
  marigold circular medallion showing '6.5', headline 'Good progress!',
  caption 'Estimated overall band · IELTS-style practice', a 'Share result'
  outline pill; a card with four criteria rows each with a teal progress bar
  and a quoted example; correction cards with red strikethrough 'you said'
  and green bold 'better' plus a 'Explain in Urdu' toggle showing an Urdu RTL
  box; two stat cards (words per minute, fillers); a soft teal 'Practise
  next' card."

### 7.11 Mock result (new)

- **Purpose:** the mock's payoff. Band first, then the top 3 fixes, then share.
- **Layout:** the same as 7.10, with these differences:
  - Hero caption "Estimated overall band · IELTS-style mock".
  - **"Share result"** button. It shares: "I got an estimated band 6.5 in an
    IELTS-style speaking mock on Voclara. Practise speaking with me:
    play.google.com/…".
  - **"YOUR PART 2 TASK · TRAVEL"** soft card, with the cue card prompt in
    bold.
  - The fixes section is titled **"Top 3 fixes"** and shows exactly 3.
  - **Share card image** (server-made PNG, 1080 × 1350):
    - Deep teal gradient with a marigold glow behind the medallion.
    - "Voclara" and "IELTS-style mock" at the top.
    - The big marigold medallion with "6.5 · overall band".
    - "My estimated speaking band" and "Topic: Travel".
    - Four criteria rows with bars.
    - Footer: "Practise speaking English on Voclara", the store link and
      "IELTS-style practice. Not affiliated with IELTS."
- **Planned:** "Try another mock" cta, comparison with the previous mock
  ("+0.5 since last time").
- **Stitch prompt:** "IELTS-style mock result: hero medallion '6.5', caption
  'Estimated overall band · IELTS-style mock', a 'Share result' pill, a soft
  teal card 'YOUR PART 2 TASK · TRAVEL' with the cue card sentence, four
  criteria bars with quotes, exactly three correction cards titled 'Top 3
  fixes', and a full-width 'Practise again' gradient button. Also design the
  1080×1350 shareable result image in the same style."

### 7.12 Interview setup

- **Purpose:** choose a role for the interview simulator.
- **Layout:**
  - Header (interview iris gradient): "Practise your interview" and "A
    friendly hiring manager asks 8–10 questions and follows up on what you
    say."
  - Section "CHOOSE THE ROLE": a 2-column grid of role cards (icon + title):
    Software engineer, Sales executive, Banking, Nursing, Call centre agent.
    The selected card gets a primary border and a check.
  - A Notice: "Answer in full sentences. Use examples from your studies or
    work."
  - The StartPanel (interview, 15 min).
- **Stitch prompt:** "Interview setup: indigo gradient header card, a 2-column
  grid of selectable role cards with icons (laptop, briefcase, bank, medkit,
  headset), one selected with a teal border and check, an info banner, and
  the start panel with a sunrise gradient 'Start session' button."

### 7.13 Progress (tab)

- **Purpose:** show improvement over time.
- **Layout:**
  - Title "Progress", subtitle "Your estimated bands over time" and a wash.
  - Three StatWidgets: "6.5 · Avg of last 5" (ribbon), "12 · Graded" (check)
    and "4 · Best streak" (flame).
  - A **band chart card**: vertical bars for recent sessions on a 0–9 scale.
    Bars use the teal gradient, the latest uses the sunrise gradient with its
    label in teal, and demo grades are flat grey.
  - Section "HISTORY": ListRows such as "IELTS-style mock · 6.5 · 23 Nov" and
    "Talk · demo · 22 Nov", which open the past result.
- **Empty:** an EmptyState with a stats icon, "No bands yet", "Finish a
  session of a few minutes and your first estimated band appears here." and
  the button "Start practising".
- **Stitch prompt:** "Progress tab: three small stat cards, a card with a
  vertical bar chart of bands (0–9, last bar highlighted in marigold), and a
  history list with mode icons, band values and dates; dark mode version on
  #071413 with #0E201E cards."

### 7.14 Profile (tab)

- **Purpose:** account, minutes and settings.
- **Layout:**
  - Title "Profile" with a wash.
  - **Account card** (brand gradient): avatar circle, name (or "Add your
    name"), phone number and a pill "Free plan".
  - **Practice minutes card:** "Practice minutes" with "10 min left" in teal,
    a ProgressBar and the caption "Free drills today: 2 min. Plans and
    top-ups are coming soon."
  - Section "LEARNING": ListGroup with "Explanation language · Urdu ›" and
    "Level & goal · B1 · IELTS ›".
  - Section "APP": ListGroup with "About & privacy ›" and "Sign out" (coral).
- **Planned:** a "Plans & minutes" row with a marigold "Upgrade" badge, and a
  "Notifications" row.
- **Stitch prompt:** "Profile tab: a teal gradient account card with avatar,
  name, phone and a 'Free plan' pill; a minutes card with a teal progress
  bar; grouped settings list rows with tinted icon circles and chevrons; a red
  'Sign out' row."

### 7.15 Settings: Explanation language

- Intro line: "Corrections are explained in English and in this language."
- Selectable rows, each with an English name and the native script on the
  right: **Urdu / اردو** (selected, with a teal check and border), Hindi /
  हिन्दी, Bengali / বাংলা, Arabic / العربية, English only.
- Choosing a row saves it straight away.
- **Planned:** a preview card showing a sample explanation in the chosen
  language.
- **Stitch prompt:** "Language picker: a short intro line and a list of
  selectable cards, each with the English language name and its native script
  (Urdu in Nastaliq) on the right; Urdu selected with a teal border and
  check."

### 7.16 Settings: Level & goal ("You & your goal")

- Field "YOUR NAME" (placeholder "e.g. Ayesha").
- "YOUR ENGLISH LEVEL" chips A1 A2 **B1** B2 C1 C2, with a description line
  ("Intermediate. Sana matches her speed and vocabulary to this.").
- "YOUR MAIN GOAL" as a list of selectable rows with icons: IELTS, Job
  interviews, English at work, Moving abroad, **Daily conversation** (selected
  with a check).
- A **"Save"** button at the bottom.
- **Stitch prompt:** "Profile settings form: a name input, CEFR level chips
  A1–C2 with B1 selected, a stacked list of goal options with icons (one
  selected with a teal check and border), and a full-width save button."

### 7.17 About & privacy

- The app name "Voclara" and "Version 0.1.0".
- Four point cards, each with an icon, a title and a body:
  - **Your voice:** "Used only to run your session and give feedback.
    Recordings are deleted after 30 days."
  - **No AI training:** "We never use your audio or transcripts to train AI
    models."
  - **Delete anytime:** "Deleting your account removes your sessions, scores
    and recordings."
  - **Estimated bands:** "Bands are IELTS-style estimates for practice.
    Voclara is not affiliated with IELTS."
- **Planned:** links to Terms, Privacy and Contact.

### 7.18 Paywall: Plans & minutes (planned)

- **Purpose:** convert when minutes run out or the user wants mocks.
- **Layout:**
  - Header with the orb and "Keep speaking every day".
  - Three plan cards:
    - **Starter:** 60 min/month.
    - **Pro:** 300 min/month plus 4 mocks. "Most popular" marigold ribbon,
      selected.
    - **Mock pack:** 3 mocks, one-time.
  - Prices in PKR (shown as "Rs 999 / month"), with Google Play and local
    payment logos as text chips.
  - A cta "Continue", the ghost "Restore purchase" and small terms.
- **Stitch prompt:** "Subscription paywall: glowing orb header 'Keep speaking
  every day', three selectable plan cards (middle one highlighted with a
  marigold 'Most popular' ribbon), PKR prices, feature checklists, a sunrise
  gradient 'Continue' button and a 'Restore purchase' text button."

### 7.19 Pronunciation drill (planned)

- **Purpose:** 2-minute sound drills (v/w, th, short vowels).
- **Layout:**
  - Drill header (drill coral gradient): "v vs w" with "3 of 8".
  - The target word card: big word "**very well**", an IPA hint and a "Listen"
    button that plays the model.
  - The hold-to-talk button.
  - Result: each word coloured by score (green ≥ 80, marigold 60–79, coral
    < 60), with a percentage and "Try again" / "Next".
- **Stitch prompt:** "Pronunciation drill: coral gradient header 'v vs w ·
  3 of 8', a large word card 'very well' with a listen button and phonetic
  hint, a round mic button, and a result row where each word is coloured by
  score with percentages."

### 7.20 Streaks and reminders (planned)

- A streak sheet: a big marigold flame with "4-day streak", a week row of 7
  circles (done ones filled marigold), and the copy "Speak for 2 minutes today
  to keep it going."
- A push notification style for daily reminders: "Sana: 2 minutes of English
  today? 🙂".

### 7.21 Placement test (planned)

- A 3-question spoken check at sign-up. It shows a progress bar, "Question 1
  of 3", the orb, and ends with "Your level: B1 · Intermediate", with the
  button "Looks right" or "Change".

---

## 8. Content and copy bank

- **Greetings:** "Good morning", "Good afternoon", "Good evening", "Ready to
  speak? 👋".
- **Encouragement by band:** ≥ 7 "Excellent work!" · ≥ 6 "Good progress!" ·
  ≥ 5 "Nice effort, keep going!" · below 5 "Every session makes you
  stronger."
- **Tips:**
  - "Record a 2-minute talk daily. Small, regular practice beats one long
    session."
  - "Answer with a reason and an example."
  - "It's fine to pause. Fillers like 'um' cost more than silence."
- **Sample cue cards** (there are 30+ in the app), with their themes:
  - "Describe a teacher who has influenced you." (Education)
  - "Describe a memorable journey you have made." (Travel)
  - "Describe a festival or celebration that is important in your country."
    (Culture)
  - "Describe an app or website that you find very useful." (Technology)
  - "Describe a time when you had to use English to communicate." (Languages)
- **Sample correction:** said "I am working there since two years" → better
  "I have been working there for two years". Why (EN): "Use the present
  perfect continuous for something that started in the past and continues."
- **Disclaimer** (everywhere a band appears): "Estimated band · IELTS-style
  practice".

## 9. Do and don't

| ✅ Do | ❌ Don't |
| --- | --- |
| One marigold CTA per screen | Two gradient buttons fighting for attention |
| Big, calm, centred speaking UI on a dark canvas | Busy dashboards during a live session |
| Show the band, then evidence, then a few fixes | Walls of red corrections |
| Use the orb and abstract shapes | Human stock photos, cartoon mascots, flags |
| "Estimated", "IELTS-style" | "Your IELTS score", official logos |
| Urdu only inside explanation boxes (RTL) | Hard-coding Urdu into the interface |
| Use the 4-pt spacing and radius tokens | Arbitrary sizes and colours |
