/**
 * Core building blocks: Txt, Screen, Gradient, Icon, Card, Button, IconButton.
 * Rules: apps/mobile/DESIGN.md. Every value comes from the theme tokens.
 */
import Ionicons from '@expo/vector-icons/Ionicons';
import { type ComponentProps, type ReactNode } from 'react';
import {
  ActivityIndicator,
  Animated,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
  type PressableProps,
  type StyleProp,
  type TextProps,
  type TextStyle,
  type ViewStyle,
} from 'react-native';
import { useAnimatedValue } from './anim';
import { SafeAreaView, type Edge } from 'react-native-safe-area-context';

import { useTheme, type Gradients, type Theme } from '@/theme';
import type { TypeVariant } from '@/theme/tokens';

// ---------------------------------------------------------------- Txt

export type Tone =
  | 'default'
  | 'muted'
  | 'primary'
  | 'accent'
  | 'danger'
  | 'success'
  | 'info'
  | 'onPrimary'
  | 'onCanvas'
  | 'onCanvasMuted'
  | 'onBrand'
  | 'onBrandMuted'
  | 'onAccent'
  | 'onAccentMuted';

export function toneColor(t: Theme, tone: Tone): string {
  const c = t.colors;
  return {
    default: c.text,
    muted: c.textMuted,
    primary: c.primary,
    accent: c.accent,
    danger: c.danger,
    success: c.success,
    info: c.info,
    onPrimary: c.onPrimary,
    onCanvas: c.onCanvas,
    onCanvasMuted: c.onCanvasMuted,
    onBrand: c.onBrand,
    onBrandMuted: c.onBrandMuted,
    onAccent: c.onAccent,
    onAccentMuted: c.onAccentMuted,
  }[tone];
}

interface TxtProps extends TextProps {
  variant?: TypeVariant;
  tone?: Tone;
  align?: TextStyle['textAlign'];
  children?: ReactNode;
}

/** All text in the app. */
export function Txt({ variant = 'body', tone = 'default', align, style, ...rest }: TxtProps) {
  const t = useTheme();
  return (
    <Text
      style={[t.type[variant], { color: toneColor(t, tone), textAlign: align }, style]}
      {...rest}
    />
  );
}

// ---------------------------------------------------------------- Gradient

interface GradientProps {
  name: keyof Gradients;
  style?: StyleProp<ViewStyle>;
  children?: ReactNode;
}

/** A view painted with a theme gradient (native `backgroundImage`, no package). */
export function Gradient({ name, style, children }: GradientProps) {
  const t = useTheme();
  return <View style={[{ backgroundImage: t.gradients[name], overflow: 'hidden' }, style]}>{children}</View>;
}

// ---------------------------------------------------------------- Icon

export type IconName = ComponentProps<typeof Ionicons>['name'];

export function Icon({ name, size = 22, color }: { name: IconName; size?: number; color?: string }) {
  const t = useTheme();
  return <Ionicons name={name} size={size} color={color ?? t.colors.text} />;
}

// ---------------------------------------------------------------- Screen

interface ScreenProps {
  children: ReactNode;
  scroll?: boolean;
  /** Large title shown at the top (tab screens hide the native header). */
  title?: string;
  subtitle?: string;
  /** Element at the right of the title (e.g. an avatar). */
  trailing?: ReactNode;
  edges?: Edge[];
  padded?: boolean;
  /** Soft brand wash behind the header area. */
  wash?: boolean;
}

export function Screen({
  children,
  scroll = true,
  title,
  subtitle,
  trailing,
  edges = ['left', 'right'],
  padded = true,
  wash = false,
}: ScreenProps) {
  const t = useTheme();
  const header = title ? (
    <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.md, marginBottom: t.space.xs }}>
      <View style={{ flex: 1, gap: t.space.xxs }}>
        <Txt variant="title" accessibilityRole="header">
          {title}
        </Txt>
        {subtitle ? <Txt tone="muted">{subtitle}</Txt> : null}
      </View>
      {trailing}
    </View>
  ) : null;
  const contentStyle: ViewStyle = {
    padding: padded ? t.space.lg : 0,
    paddingBottom: t.space.huge,
    gap: t.space.lg,
  };
  return (
    <SafeAreaView edges={edges} style={{ flex: 1, backgroundColor: t.colors.bg }}>
      {wash && (
        <View
          pointerEvents="none"
          style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 260, backgroundImage: t.gradients.wash }}
        />
      )}
      {scroll ? (
        <ScrollView contentContainerStyle={contentStyle} keyboardShouldPersistTaps="handled">
          {header}
          {children}
        </ScrollView>
      ) : (
        <View style={[contentStyle, { flex: 1 }]}>
          {header}
          {children}
        </View>
      )}
    </SafeAreaView>
  );
}

// ---------------------------------------------------------------- pressable scale

/** Shared press feedback: a quick, subtle shrink. */
function usePressScale() {
  const t = useTheme();
  const scale = useAnimatedValue(1);
  const to = (v: number) =>
    Animated.timing(scale, { toValue: v, duration: t.motion.fast, useNativeDriver: true }).start();
  return { scale, onPressIn: () => to(0.97), onPressOut: () => to(1) };
}

// ---------------------------------------------------------------- Card

interface CardProps {
  children: ReactNode;
  tone?: 'default' | 'soft' | 'outline';
  onPress?: () => void;
  style?: StyleProp<ViewStyle>;
  accessibilityLabel?: string;
}

export function Card({ children, tone = 'default', onPress, style, accessibilityLabel }: CardProps) {
  const t = useTheme();
  const press = usePressScale();
  const body: ViewStyle = {
    backgroundColor: tone === 'soft' ? t.colors.primarySoft : tone === 'outline' ? 'transparent' : t.colors.surface,
    borderRadius: t.radius.lg,
    borderWidth: tone === 'outline' ? 1 : 0,
    borderColor: t.colors.border,
    padding: t.space.lg,
    gap: t.space.md,
    boxShadow: tone === 'default' ? t.shadows.card : t.shadows.none,
  };
  if (!onPress) return <View style={[body, style]}>{children}</View>;
  // The Pressable is the flex child of the parent row, so it must carry the
  // card's own flex/width; the inner view then fills it.
  const flat = StyleSheet.flatten(style) ?? {};
  const outer: ViewStyle = { flex: flat.flex, flexGrow: flat.flexGrow, width: flat.width, alignSelf: flat.alignSelf };
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={accessibilityLabel}
      onPress={onPress}
      onPressIn={press.onPressIn}
      onPressOut={press.onPressOut}
      style={outer}
    >
      <Animated.View style={[body, style, { flex: flat.flex ? 1 : undefined, transform: [{ scale: press.scale }] }]}>
        {children}
      </Animated.View>
    </Pressable>
  );
}

// ---------------------------------------------------------------- Button

type ButtonVariant = 'primary' | 'cta' | 'secondary' | 'ghost' | 'danger';

interface ButtonProps extends Omit<PressableProps, 'children' | 'style'> {
  title: string;
  variant?: ButtonVariant;
  size?: 'md' | 'lg';
  icon?: IconName;
  loading?: boolean;
  style?: StyleProp<ViewStyle>;
}

export function Button({
  title,
  variant = 'primary',
  size = 'md',
  icon,
  loading,
  disabled,
  style,
  ...rest
}: ButtonProps) {
  const t = useTheme();
  const press = usePressScale();
  const c = t.colors;
  const fg =
    variant === 'cta' ? c.onAccent : variant === 'primary' || variant === 'danger' ? c.onPrimary : variant === 'secondary' ? c.text : c.primary;
  const box: ViewStyle = {
    minHeight: size === 'lg' ? 58 : t.touch.min + 4,
    borderRadius: t.radius.pill,
    paddingHorizontal: t.space.xxl,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: t.space.sm,
    backgroundColor:
      variant === 'primary' ? c.primary : variant === 'danger' ? c.danger : variant === 'secondary' ? c.surface : 'transparent',
    backgroundImage: variant === 'cta' ? t.gradients.sunrise : undefined,
    borderWidth: variant === 'secondary' ? 1 : 0,
    borderColor: c.border,
    boxShadow: variant === 'cta' && !disabled ? t.shadows.glow : t.shadows.none,
    opacity: disabled ? 0.5 : 1,
  };
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled: !!disabled, busy: !!loading }}
      disabled={disabled || loading}
      onPressIn={press.onPressIn}
      onPressOut={press.onPressOut}
      {...rest}
    >
      <Animated.View style={[box, style, { transform: [{ scale: press.scale }] }]}>
        {loading ? (
          <ActivityIndicator color={fg} />
        ) : (
          <>
            {icon && <Icon name={icon} size={20} color={fg} />}
            <Txt variant="bodyStrong" style={{ color: fg }}>
              {title}
            </Txt>
          </>
        )}
      </Animated.View>
    </Pressable>
  );
}

// ---------------------------------------------------------------- IconButton

interface IconButtonProps {
  icon: IconName;
  /** Required: read by screen readers. */
  label: string;
  onPress: () => void;
  tone?: 'surface' | 'ghost' | 'canvas';
  size?: number;
}

export function IconButton({ icon, label, onPress, tone = 'surface', size = 22 }: IconButtonProps) {
  const t = useTheme();
  const press = usePressScale();
  const bg = tone === 'surface' ? t.colors.surface : tone === 'canvas' ? t.colors.glass : 'transparent';
  const fg = tone === 'canvas' ? t.colors.onCanvas : t.colors.text;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={onPress}
      onPressIn={press.onPressIn}
      onPressOut={press.onPressOut}
      hitSlop={6}
    >
      <Animated.View
        style={{
          width: t.touch.min,
          height: t.touch.min,
          borderRadius: t.radius.pill,
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: bg,
          boxShadow: tone === 'surface' ? t.shadows.card : t.shadows.none,
          transform: [{ scale: press.scale }],
        }}
      >
        <Icon name={icon} size={size} color={fg} />
      </Animated.View>
    </Pressable>
  );
}
