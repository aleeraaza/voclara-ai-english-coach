/**
 * Voclara design tokens: "Indus & Marigold".
 *
 * The ONLY place raw colours, sizes, radii, shadows, gradients and timings are
 * defined. Screens and components read them through `useTheme()` and never
 * hard-code a hex value or a magic number. Rules: apps/mobile/DESIGN.md.
 *
 * - Indus teal: the brand. Calm, trustworthy, a "clear voice".
 * - Marigold (genda): warmth and celebration. The single accent for wins,
 *   streaks and the one primary call to action per screen.
 * - Ivory paper in light mode, deep ink-teal at night.
 */

export type ColorScheme = 'light' | 'dark';

/** Raw palette. Do not use directly in screens: use `colors` roles below. */
export const palette = {
  indus50: '#EAF6F4',
  indus100: '#D3EEEA',
  indus200: '#A6DCD5',
  indus300: '#6FC4BA',
  indus400: '#35A89C',
  indus500: '#0E8C80',
  indus600: '#0B7369',
  indus700: '#095B53',
  indus800: '#073F3A',
  indus900: '#05302C',
  indus950: '#031E1C',

  marigold100: '#FDEDD3',
  marigold300: '#F8C878',
  marigold400: '#F5B14A',
  marigold500: '#EE9B1F',
  marigold700: '#9A5B05',

  coral100: '#FCE2DF',
  coral500: '#E2554C',
  coral300: '#F08A82',
  leaf100: '#DCF3E6',
  leaf500: '#23935E',
  leaf300: '#5CC791',
  iris100: '#E6E7FB',
  iris500: '#5560D6',
  iris300: '#8F96EC',

  ivory: '#F7F5EF',
  paper: '#FFFFFF',
  sand: '#ECE9E0',
  stone: '#DCD8CC',
  ink: '#0E1E1C',
  slate: '#55655F',

  night: '#071413',
  nightRaised: '#0E201E',
  nightHigh: '#152D2A',
  nightLine: '#22403B',
  mist: '#E8F1EF',
  fog: '#94AAA5',
} as const;

/** Semantic colour roles. Every colour on screen comes from one of these. */
export interface Colors {
  /** Screen background. */
  bg: string;
  /** Cards, sheets, inputs. */
  surface: string;
  /** Subtle fills: chips, tracks, pressed rows. */
  surfaceAlt: string;
  /** Hairlines and input borders. */
  border: string;
  text: string;
  /** Secondary text. Never for body copy under 14 px on surfaceAlt. */
  textMuted: string;
  /** Text and icons on `primary` / `brand` gradients. */
  onPrimary: string;
  primary: string;
  primaryPressed: string;
  /** Tinted background behind primary icons and selected chips. */
  primarySoft: string;
  /** Marigold: one per screen, for the main win/celebration or CTA glow. */
  accent: string;
  onAccent: string;
  accentSoft: string;
  success: string;
  successSoft: string;
  danger: string;
  dangerSoft: string;
  /** Secondary data colour for charts and the "interview" mode. */
  info: string;
  infoSoft: string;
  /** Scrim behind sheets and modals. */
  overlay: string;
  /** Text on the always-dark live-session canvas. */
  onCanvas: string;
  onCanvasMuted: string;
  /** Text and icons on the brand/mode gradients (dark in both themes). */
  onBrand: string;
  onBrandMuted: string;
  /** Secondary text on marigold surfaces. */
  onAccentMuted: string;
  /** Translucent fills on the canvas (buttons, chips, tutor bubbles). */
  glass: string;
  glassStrong: string;
  /** The learner's caption bubble on the canvas. */
  learnerBubble: string;
}

export const colors: Record<ColorScheme, Colors> = {
  light: {
    bg: palette.ivory,
    surface: palette.paper,
    surfaceAlt: palette.sand,
    border: palette.stone,
    text: palette.ink,
    textMuted: palette.slate,
    onPrimary: palette.paper,
    primary: palette.indus600,
    primaryPressed: palette.indus700,
    primarySoft: palette.indus50,
    accent: palette.marigold500,
    onAccent: '#2A1900',
    accentSoft: palette.marigold100,
    success: palette.leaf500,
    successSoft: palette.leaf100,
    danger: palette.coral500,
    dangerSoft: palette.coral100,
    info: palette.iris500,
    infoSoft: palette.iris100,
    overlay: 'rgba(7, 20, 19, 0.55)',
    onCanvas: palette.mist,
    onCanvasMuted: palette.fog,
    onBrand: '#FFFFFF',
    onBrandMuted: 'rgba(255, 255, 255, 0.84)',
    onAccentMuted: 'rgba(42, 25, 0, 0.78)',
    glass: 'rgba(255, 255, 255, 0.10)',
    glassStrong: 'rgba(255, 255, 255, 0.18)',
    learnerBubble: 'rgba(111, 196, 186, 0.22)',
  },
  dark: {
    bg: palette.night,
    surface: palette.nightRaised,
    surfaceAlt: palette.nightHigh,
    border: palette.nightLine,
    text: palette.mist,
    textMuted: palette.fog,
    onPrimary: palette.indus950,
    primary: palette.indus300,
    primaryPressed: palette.indus200,
    primarySoft: '#123A36',
    accent: palette.marigold400,
    onAccent: '#2A1900',
    accentSoft: '#3A2A10',
    success: palette.leaf300,
    successSoft: '#133B29',
    danger: palette.coral300,
    dangerSoft: '#3D1C1A',
    info: palette.iris300,
    infoSoft: '#20244A',
    overlay: 'rgba(0, 0, 0, 0.6)',
    onCanvas: palette.mist,
    onCanvasMuted: palette.fog,
    onBrand: '#FFFFFF',
    onBrandMuted: 'rgba(255, 255, 255, 0.84)',
    onAccentMuted: 'rgba(42, 25, 0, 0.78)',
    glass: 'rgba(255, 255, 255, 0.10)',
    glassStrong: 'rgba(255, 255, 255, 0.18)',
    learnerBubble: 'rgba(111, 196, 186, 0.22)',
  },
};

/**
 * Gradients as CSS strings for the native `backgroundImage` style
 * (React Native 0.76+; no extra package). Use through `<Gradient>`.
 */
export interface Gradients {
  /** Hero cards and the splash: deep teal into bright teal. */
  brand: string;
  /** The live-session canvas (always dark). */
  canvas: string;
  /** Warm glow behind wins and the primary CTA. */
  sunrise: string;
  /** Soft tint for screen headers. */
  wash: string;
  /** The voice orb. */
  orb: string;
  /** Mode tiles. */
  talk: string;
  mock: string;
  interview: string;
  drill: string;
}

export const gradients: Record<ColorScheme, Gradients> = {
  light: {
    brand: `linear-gradient(135deg, ${palette.indus800} 0%, ${palette.indus600} 55%, ${palette.indus400} 100%)`,
    canvas: `radial-gradient(circle at 50% 35%, ${palette.indus800} 0%, ${palette.night} 70%)`,
    sunrise: `linear-gradient(135deg, ${palette.marigold400} 0%, ${palette.coral500} 100%)`,
    wash: `linear-gradient(180deg, ${palette.indus100} 0%, ${palette.ivory} 100%)`,
    orb: `radial-gradient(circle at 35% 30%, ${palette.marigold300} 0%, ${palette.indus400} 45%, ${palette.indus700} 100%)`,
    talk: `linear-gradient(135deg, ${palette.indus500} 0%, ${palette.indus700} 100%)`,
    mock: `linear-gradient(135deg, ${palette.marigold400} 0%, ${palette.marigold500} 100%)`,
    interview: `linear-gradient(135deg, ${palette.iris300} 0%, ${palette.iris500} 100%)`,
    drill: `linear-gradient(135deg, ${palette.coral300} 0%, ${palette.coral500} 100%)`,
  },
  dark: {
    brand: `linear-gradient(135deg, ${palette.indus900} 0%, ${palette.indus700} 55%, ${palette.indus500} 100%)`,
    canvas: `radial-gradient(circle at 50% 35%, ${palette.indus800} 0%, ${palette.night} 70%)`,
    sunrise: `linear-gradient(135deg, ${palette.marigold400} 0%, ${palette.coral500} 100%)`,
    wash: `linear-gradient(180deg, ${palette.nightHigh} 0%, ${palette.night} 100%)`,
    orb: `radial-gradient(circle at 35% 30%, ${palette.marigold300} 0%, ${palette.indus400} 45%, ${palette.indus800} 100%)`,
    talk: `linear-gradient(135deg, ${palette.indus400} 0%, ${palette.indus700} 100%)`,
    mock: `linear-gradient(135deg, ${palette.marigold400} 0%, ${palette.marigold500} 100%)`,
    interview: `linear-gradient(135deg, ${palette.iris300} 0%, ${palette.iris500} 100%)`,
    drill: `linear-gradient(135deg, ${palette.coral300} 0%, ${palette.coral500} 100%)`,
  },
};

/** 4-point grid. Use these, never arbitrary padding. */
export const space = {
  xxs: 2,
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 20,
  xxl: 24,
  xxxl: 32,
  huge: 48,
} as const;

export const radius = {
  sm: 10,
  md: 14,
  lg: 20,
  xl: 28,
  pill: 999,
} as const;

/** Font families (Plus Jakarta Sans, loaded at startup in the root layout). */
export const fonts = {
  regular: 'PlusJakartaSans_400Regular',
  medium: 'PlusJakartaSans_500Medium',
  semibold: 'PlusJakartaSans_600SemiBold',
  bold: 'PlusJakartaSans_700Bold',
  extrabold: 'PlusJakartaSans_800ExtraBold',
} as const;

/** Type scale. Use through `<Txt variant>`. */
export const type = {
  display: { fontFamily: fonts.extrabold, fontSize: 34, lineHeight: 40, letterSpacing: -0.6 },
  title: { fontFamily: fonts.bold, fontSize: 24, lineHeight: 30, letterSpacing: -0.3 },
  heading: { fontFamily: fonts.bold, fontSize: 18, lineHeight: 24, letterSpacing: -0.1 },
  body: { fontFamily: fonts.medium, fontSize: 16, lineHeight: 24 },
  bodyStrong: { fontFamily: fonts.bold, fontSize: 16, lineHeight: 24 },
  small: { fontFamily: fonts.medium, fontSize: 14, lineHeight: 20 },
  caption: { fontFamily: fonts.medium, fontSize: 12, lineHeight: 16 },
  label: { fontFamily: fonts.bold, fontSize: 12, lineHeight: 16, letterSpacing: 0.8, textTransform: 'uppercase' as const },
  /** Big numbers: bands, timers, minutes. */
  metric: { fontFamily: fonts.extrabold, fontSize: 44, lineHeight: 48, letterSpacing: -1, fontVariant: ['tabular-nums' as const] },
} as const;

export type TypeVariant = keyof typeof type;

/** Elevation as native `boxShadow` strings (React Native 0.76+). */
export const shadows: Record<ColorScheme, { card: string; raised: string; glow: string; none: string }> = {
  light: {
    none: 'none',
    card: '0px 1px 2px rgba(14, 30, 28, 0.06), 0px 4px 14px rgba(14, 30, 28, 0.06)',
    raised: '0px 8px 24px rgba(14, 30, 28, 0.14)',
    glow: `0px 10px 30px ${palette.marigold500}66`,
  },
  dark: {
    none: 'none',
    card: '0px 1px 0px rgba(255, 255, 255, 0.03)',
    raised: '0px 10px 28px rgba(0, 0, 0, 0.45)',
    glow: `0px 10px 30px ${palette.marigold400}44`,
  },
};

/** Motion. Keep it calm: nothing loops faster than `breath`. */
export const motion = {
  fast: 150,
  base: 250,
  slow: 400,
  /** Splash wordmark entrance. */
  intro: 700,
  /** Idle orb breathing / sound-wave cycle. */
  breath: 1400,
  /** Minimum time the animated splash stays up (so it never flickers). */
  splashMin: 1400,
} as const;

/** Minimum touch target (WCAG 2.5.5 / Android 48 dp). */
export const touch = { min: 48 } as const;
