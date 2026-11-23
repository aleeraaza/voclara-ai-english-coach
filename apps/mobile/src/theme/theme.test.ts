import { colors, type ColorScheme } from './tokens';

/** WCAG 2.x relative luminance of a #RRGGBB colour. */
function luminance(hex: string): number {
  const n = parseInt(hex.slice(1), 16);
  const ch = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * ch[0]! + 0.7152 * ch[1]! + 0.0722 * ch[2]!;
}

export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

/** Text/background pairs the UI actually uses, with the WCAG AA minimum. */
const PAIRS: [keyof (typeof colors)['light'], keyof (typeof colors)['light'], number][] = [
  ['text', 'bg', 4.5],
  ['text', 'surface', 4.5],
  ['text', 'surfaceAlt', 4.5],
  ['textMuted', 'bg', 4.5],
  ['textMuted', 'surface', 4.5],
  ['onPrimary', 'primary', 4.5],
  ['onAccent', 'accent', 4.5],
  ['primary', 'surface', 3], // icons, links, large text
  ['primary', 'primarySoft', 3],
  ['danger', 'surface', 3],
  ['success', 'surface', 3],
  ['info', 'surface', 3],
];

describe.each(['light', 'dark'] as ColorScheme[])('%s theme', (scheme) => {
  const c = colors[scheme];
  it.each(PAIRS)('%s on %s meets %s:1', (fg, bg, min) => {
    const ratio = contrast(c[fg], c[bg]);
    expect(ratio).toBeGreaterThanOrEqual(min);
  });
});
