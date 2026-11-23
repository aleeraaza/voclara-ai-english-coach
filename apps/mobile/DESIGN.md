# Voclara design system — "Indus & Marigold"

Read this before changing any screen. It is the contract for humans and AI
agents. Tokens live in [`src/theme/tokens.ts`](src/theme/tokens.ts);
components in [`src/components/`](src/components/). If something you need is
missing, **add it to the tokens or the kit first**, then use it.

## 1. Principles

1. **One thing per screen.** Each screen has one job and at most one primary
   (marigold-glow or brand) call to action. Secondary paths are tiles, rows or
   tabs that lead to their own screen. Do not stack features on one screen.
2. **Voice first.** The live session is an immersive, dark, calm canvas: an
   orb, captions and one big talk button. No chrome competes with speaking.
3. **Warm and encouraging.** Learners are nervous speakers. Copy is short,
   kind and specific ("Nice! One fix to try"). Never shame a mistake.
4. **Clear before clever.** Plain words, large touch targets (48 dp min),
   real contrast (WCAG AA, enforced by `src/theme/theme.test.ts`).
5. **Calm motion.** Animations explain state (listening, speaking, loading).
   Nothing flashes or loops faster than `motion.breath`.

## 2. Tokens (never hard-code values)

| Category | Where | Rule |
| --- | --- | --- |
| Colour | `colors[scheme]` roles | Use roles (`surface`, `primary`, `textMuted`…), never hex in screens. `palette` is private to tokens.ts. |
| Gradients | `gradients[scheme]` | CSS strings for native `backgroundImage`; render with `<Gradient name>`. |
| Type | `type` + `<Txt variant>` | display / title / heading / body / bodyStrong / small / caption / label / metric. Font: Plus Jakarta Sans. |
| Spacing | `space` | 4-pt grid: xs 4, sm 8, md 12, lg 16, xl 20, xxl 24, xxxl 32, huge 48. Screen padding = `lg`. |
| Radius | `radius` | sm 10 (chips), md 14 (inputs, small cards), lg 20 (cards), xl 28 (hero, sheets), pill. |
| Elevation | `shadows[scheme]` | `card` for resting cards, `raised` for floating UI, `glow` only behind the primary CTA. |
| Motion | `motion` | fast 150 (press), base 250 (state), slow 400 (enter), breath 1400 (idle loops). |

### Colour roles at a glance

| Role | Light | Dark | Use |
| --- | --- | --- | --- |
| `primary` (Indus) | #0B7369 | #6FC4BA | Brand, links, selected state, progress |
| `accent` (Marigold) | #EE9B1F | #F5B14A | Streaks, wins, the one hero CTA glow |
| `bg` | #F7F5EF ivory | #071413 night | Screen background |
| `surface` | #FFFFFF | #0E201E | Cards, sheets, inputs |
| `success` / `danger` | leaf / coral | lighter tints | Corrections (said vs better), states |
| `info` (Iris) | #5560D6 | #8F96EC | Charts, interview mode |

The live-session screen always uses the dark `gradients.canvas` with
`colors.onCanvas` text, in both themes.

## 3. Components (`src/components`)

| Component | Variants / props | Use when | States |
| --- | --- | --- | --- |
| `Screen` | `scroll`, `padded`, `header` | Every screen root. Handles safe area and background. | — |
| `Txt` | `variant`, `tone` (default, muted, primary, accent, danger, success, onPrimary, onCanvas) | All text. Never `<Text>` with ad-hoc styles. | — |
| `Button` | `primary` (brand), `cta` (marigold, glow), `secondary` (outline), `ghost`, `danger`; `size` md/lg; `icon`; `loading` | Actions. One `cta` per screen at most. | pressed (scale 0.98), disabled (50% opacity), loading (spinner) |
| `IconButton` | `tone`, `size` | Toolbar actions (close, back, settings). Always pass `label`. | pressed |
| `Card` | `tone` (default, soft, outline), `onPress` | Grouping content; tappable cards navigate. Layout props in `style` (flex, width) apply to the tappable wrapper too. | pressed |
| `Gradient` | `name` (a gradients key), `radius` | Hero cards, mode tiles, splash, canvas. | — |
| `ModeTile` | `mode`, `title`, `subtitle`, `badge` | Entry points to practice modes (Home, Practice). | pressed, disabled ("Soon" badge) |
| `StatWidget` | `icon`, `value`, `label`, `tone` | Small dashboard numbers (minutes, streak, band). | — |
| `ListRow` | `icon`, `title`, `subtitle`, `value`, `chevron` | Settings and history lists. | pressed |
| `Chip` / `ChipGroup` | `selected` | Single choice among short options. | selected, pressed |
| `ProgressBar` | `value` 0–1, `tone` | Bands, minutes, criteria. | animated fill |
| `BandBadge` | `band`, `size` | The estimated band, always labelled "IELTS-style estimate". | — |
| `VoiceOrb` | `state` (idle, listening, speaking), `level`, `size` | The tutor's presence: splash, onboarding, live session. | breathes when idle; pulses with level |
| `SoundWave` | `bars`, `active`, `tone` | Splash and "listening" indicators. | animated |
| `EmptyState` | `icon`, `title`, `body`, `action` | Lists with nothing yet. | — |
| `Field` | `label`, `hint`, `error` | Text input. | focused (primary border), error (danger border + message) |

Do's and don'ts:

| ✅ Do | ❌ Don't |
| --- | --- |
| Navigate to a new screen for a new task | Put every feature on Home |
| Use `Txt` variants and colour roles | Write `fontSize: 17` or `color: '#333'` |
| Give every icon-only button an accessibility `label` | Use colour alone to show meaning (pair with text or icon) |
| Keep one marigold CTA per screen | Use marigold for errors or warnings |
| Say "estimated band (IELTS-style)" | Claim official IELTS scores or affiliation |
| Draw illustrations from tokens (`VoiceOrb`, gradients, icons) | Paste remote images of unknown licence |

## 4. Navigation patterns

```
Splash (animated) -> Onboarding (first launch) -> Sign in -> Tabs
Tabs:  Home | Practice | Progress | Profile
Home      -> hero "Talk with Sana", widgets, quick tiles -> setup screens
Practice  -> mode tiles -> /practice/talk | /practice/mock | /practice/interview -> /talk (live)
/talk     -> on end -> /feedback (polls grading)
Progress  -> band chart, history rows -> /session/[id]
Profile   -> rows -> /settings/language | /settings/learning | /settings/about
```

- Tabs are for top-level sections only (max 4). Tasks open as stack screens
  with a back button; the live session hides tabs and gestures.
- Every list row and tile leads somewhere; no dead ends. Disabled features
  show a "Soon" badge and are not tappable.

## 5. Accessibility

- Text pairs are checked for WCAG AA in `src/theme/theme.test.ts`; add any new
  pair there.
- Touch targets ≥ `touch.min` (48). Icon buttons take a `label`.
- Respect the system font scale: never set `allowFontScaling={false}`.
- Urdu/Hindi explanations use `writingDirection: 'auto'` and the system font
  (Plus Jakarta Sans has no Arabic/Devanagari glyphs).

## 6. Assets

Illustrations are built in code (`VoiceOrb`, `SoundWave`, gradients, Ionicons)
so they stay sharp, theme with dark mode and need no licences. Icons:
`@expo/vector-icons` Ionicons only (one icon family). If a raster image is
ever needed, put it in `assets/images/`, note its source and licence in
`assets/images/CREDITS.md`, and keep it under 200 KB.

## 7. Native-build impact

Gradients (`backgroundImage`), shadows (`boxShadow`), Animated, fonts and
icons need **no new native code**: the installed dev build picks them up over
Metro. Changing the native splash (`app.json` → `splash`), the app icon or
adding a native package requires a new build.

Animated values: use `useAnimatedValue` from `src/components/anim.ts`, not
`useRef(new Animated.Value(..)).current` (fails the React Compiler lint) and not
React Native's own `useAnimatedValue` (missing on react-native-web, which the
web preview uses).
