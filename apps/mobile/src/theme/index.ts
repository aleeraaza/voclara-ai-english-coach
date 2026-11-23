/**
 * Theme access. Components call `useTheme()` and read semantic roles:
 *
 *   const t = useTheme();
 *   <View style={{ backgroundColor: t.colors.surface, padding: t.space.lg }} />
 *
 * Never import `palette` in screens; it exists for tokens.ts only.
 */
import { useColorScheme } from 'react-native';

import {
  colors,
  fonts,
  gradients,
  motion,
  radius,
  shadows,
  space,
  touch,
  type,
  type ColorScheme,
  type Colors,
  type Gradients,
} from './tokens';

export interface Theme {
  scheme: ColorScheme;
  colors: Colors;
  gradients: Gradients;
  shadows: (typeof shadows)['light'];
  space: typeof space;
  radius: typeof radius;
  type: typeof type;
  fonts: typeof fonts;
  motion: typeof motion;
  touch: typeof touch;
}

export function themeFor(scheme: ColorScheme): Theme {
  return {
    scheme,
    colors: colors[scheme],
    gradients: gradients[scheme],
    shadows: shadows[scheme],
    space,
    radius,
    type,
    fonts,
    motion,
    touch,
  };
}

const light = themeFor('light');
const dark = themeFor('dark');

/** The theme for the device's light/dark setting. */
export function useTheme(): Theme {
  return useColorScheme() === 'dark' ? dark : light;
}

export { colors, fonts, gradients, motion, radius, shadows, space, touch, type };
export type { ColorScheme, Colors, Gradients };
