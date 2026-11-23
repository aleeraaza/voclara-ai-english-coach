/**
 * Widgets: ModeTile, StatWidget, ListRow, Chip(Group), ProgressBar,
 * BandBadge, EmptyState, Field, SectionTitle. Rules: apps/mobile/DESIGN.md.
 */
import { useEffect, useState, type ReactNode } from 'react';
import { Animated, Pressable, TextInput, View, type TextInputProps, type ViewStyle } from 'react-native';
import { useAnimatedValue } from './anim';

import { useTheme, type Gradients } from '@/theme';

import { Card, Gradient, Icon, Txt, type IconName, type Tone, toneColor } from './core';

// ---------------------------------------------------------------- SectionTitle

export function SectionTitle({ children, action }: { children: string; action?: ReactNode }) {
  const t = useTheme();
  return (
    <View style={{ flexDirection: 'row', alignItems: 'center', marginTop: t.space.sm }}>
      <Txt variant="label" tone="muted" style={{ flex: 1 }}>
        {children}
      </Txt>
      {action}
    </View>
  );
}

// ---------------------------------------------------------------- ModeTile

interface ModeTileProps {
  gradient: keyof Gradients;
  icon: IconName;
  title: string;
  subtitle: string;
  onPress?: () => void;
  /** e.g. "Soon" (makes the tile non-interactive) or "15 min". */
  badge?: string;
  disabled?: boolean;
  compact?: boolean;
}

/** Entry point to a practice mode. */
export function ModeTile({ gradient, icon, title, subtitle, onPress, badge, disabled, compact }: ModeTileProps) {
  const t = useTheme();
  return (
    <Card
      onPress={disabled ? undefined : onPress}
      accessibilityLabel={`${title}. ${subtitle}`}
      style={[
        { flexDirection: compact ? 'column' : 'row', alignItems: compact ? 'flex-start' : 'center' },
        disabled ? { opacity: 0.6 } : null,
        compact ? { flex: 1 } : null,
      ]}
    >
      <Gradient
        name={gradient}
        style={{ width: 52, height: 52, borderRadius: t.radius.md, alignItems: 'center', justifyContent: 'center' }}
      >
        <Icon name={icon} size={26} color={t.colors.onBrand} />
      </Gradient>
      <View style={{ flex: compact ? undefined : 1, gap: t.space.xxs }}>
        <Txt variant="heading">{title}</Txt>
        <Txt variant="small" tone="muted" numberOfLines={2}>
          {subtitle}
        </Txt>
      </View>
      {badge ? (
        <View
          style={{
            backgroundColor: disabled ? t.colors.surfaceAlt : t.colors.accentSoft,
            borderRadius: t.radius.pill,
            paddingHorizontal: t.space.sm,
            paddingVertical: t.space.xxs,
          }}
        >
          <Txt variant="caption" tone={disabled ? 'muted' : 'default'} style={{ fontFamily: t.fonts.bold }}>
            {badge}
          </Txt>
        </View>
      ) : !compact && !disabled ? (
        <Icon name="chevron-forward" size={20} color={t.colors.textMuted} />
      ) : null}
    </Card>
  );
}

// ---------------------------------------------------------------- StatWidget

interface StatWidgetProps {
  icon: IconName;
  value: string;
  label: string;
  tone?: Extract<Tone, 'primary' | 'accent' | 'info' | 'success'>;
  onPress?: () => void;
}

export function StatWidget({ icon, value, label, tone = 'primary', onPress }: StatWidgetProps) {
  const t = useTheme();
  const soft = { primary: t.colors.primarySoft, accent: t.colors.accentSoft, info: t.colors.infoSoft, success: t.colors.successSoft }[tone];
  return (
    <Card onPress={onPress} accessibilityLabel={`${label}: ${value}`} style={{ flex: 1, gap: t.space.sm }}>
      <View
        style={{ width: 36, height: 36, borderRadius: t.radius.sm, backgroundColor: soft, alignItems: 'center', justifyContent: 'center' }}
      >
        <Icon name={icon} size={20} color={toneColor(t, tone)} />
      </View>
      <Txt variant="title" style={{ fontFamily: t.fonts.extrabold }}>
        {value}
      </Txt>
      <Txt variant="caption" tone="muted">
        {label}
      </Txt>
    </Card>
  );
}

// ---------------------------------------------------------------- ListRow

interface ListRowProps {
  icon?: IconName;
  title: string;
  subtitle?: string;
  value?: string;
  onPress?: () => void;
  tone?: 'default' | 'danger';
}

export function ListRow({ icon, title, subtitle, value, onPress, tone = 'default' }: ListRowProps) {
  const t = useTheme();
  const color = tone === 'danger' ? t.colors.danger : t.colors.text;
  return (
    <Pressable
      accessibilityRole={onPress ? 'button' : undefined}
      onPress={onPress}
      style={({ pressed }) => ({
        minHeight: 60,
        flexDirection: 'row',
        alignItems: 'center',
        gap: t.space.md,
        paddingVertical: t.space.md,
        paddingHorizontal: t.space.lg,
        backgroundColor: pressed ? t.colors.surfaceAlt : 'transparent',
      })}
    >
      {icon && (
        <View
          style={{
            width: 36,
            height: 36,
            borderRadius: t.radius.sm,
            backgroundColor: tone === 'danger' ? t.colors.dangerSoft : t.colors.primarySoft,
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <Icon name={icon} size={19} color={tone === 'danger' ? t.colors.danger : t.colors.primary} />
        </View>
      )}
      <View style={{ flex: 1, gap: 2 }}>
        <Txt variant="bodyStrong" style={{ color }}>
          {title}
        </Txt>
        {subtitle ? (
          <Txt variant="small" tone="muted">
            {subtitle}
          </Txt>
        ) : null}
      </View>
      {value ? <Txt tone="muted">{value}</Txt> : null}
      {onPress && tone !== 'danger' ? <Icon name="chevron-forward" size={18} color={t.colors.textMuted} /> : null}
    </Pressable>
  );
}

/** A card holding ListRows with hairline separators. */
export function ListGroup({ children }: { children: ReactNode }) {
  const t = useTheme();
  const items = (Array.isArray(children) ? children : [children]).filter(Boolean);
  return (
    <View style={{ backgroundColor: t.colors.surface, borderRadius: t.radius.lg, overflow: 'hidden', boxShadow: t.shadows.card }}>
      {items.map((child, i) => (
        <View key={i}>
          {i > 0 && <View style={{ height: 1, backgroundColor: t.colors.border, marginLeft: 64 }} />}
          {child}
        </View>
      ))}
    </View>
  );
}

// ---------------------------------------------------------------- Chips

export function Chip({ label, selected, onPress, icon }: { label: string; selected?: boolean; onPress: () => void; icon?: IconName }) {
  const t = useTheme();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ selected: !!selected }}
      onPress={onPress}
      style={({ pressed }) => ({
        minHeight: 40,
        flexDirection: 'row',
        alignItems: 'center',
        gap: t.space.xs,
        paddingHorizontal: t.space.lg,
        borderRadius: t.radius.pill,
        borderWidth: 1,
        borderColor: selected ? t.colors.primary : t.colors.border,
        backgroundColor: selected ? t.colors.primarySoft : pressed ? t.colors.surfaceAlt : t.colors.surface,
      })}
    >
      {icon && <Icon name={icon} size={16} color={selected ? t.colors.primary : t.colors.textMuted} />}
      <Txt variant="small" style={{ fontFamily: t.fonts.semibold, color: selected ? t.colors.primary : t.colors.text }}>
        {label}
      </Txt>
    </Pressable>
  );
}

export function ChipGroup<T extends string>({
  options,
  value,
  onChange,
}: {
  options: readonly { value: T; label: string; icon?: IconName }[];
  value: T | null | undefined;
  onChange: (v: T) => void;
}) {
  const t = useTheme();
  return (
    <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: t.space.sm }}>
      {options.map((o) => (
        <Chip key={o.value} label={o.label} icon={o.icon} selected={o.value === value} onPress={() => onChange(o.value)} />
      ))}
    </View>
  );
}

// ---------------------------------------------------------------- ProgressBar

export function ProgressBar({ value, tone = 'primary', height = 8 }: { value: number; tone?: Tone; height?: number }) {
  const t = useTheme();
  const anim = useAnimatedValue(0);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    Animated.timing(anim, { toValue: Math.max(0, Math.min(1, value)), duration: t.motion.slow, useNativeDriver: false }).start();
  }, [anim, t.motion.slow, value]);
  return (
    <View
      onLayout={(e) => setWidth(e.nativeEvent.layout.width)}
      style={{ height, borderRadius: height / 2, backgroundColor: t.colors.surfaceAlt, overflow: 'hidden' }}
      accessibilityRole="progressbar"
      accessibilityValue={{ min: 0, max: 100, now: Math.round(value * 100) }}
    >
      <Animated.View
        style={{
          height,
          borderRadius: height / 2,
          backgroundColor: toneColor(t, tone),
          width: anim.interpolate({ inputRange: [0, 1], outputRange: [0, width] }),
        }}
      />
    </View>
  );
}

// ---------------------------------------------------------------- BandBadge

/** The estimated band in a gradient medallion. Always labelled as an estimate. */
export function BandBadge({ band, size = 120, label = 'IELTS-style estimate' }: { band: number | null; size?: number; label?: string }) {
  const t = useTheme();
  return (
    <View style={{ alignItems: 'center', gap: t.space.sm }} accessible accessibilityLabel={`Estimated band ${band ?? 'not available'}`}>
      <Gradient
        name="brand"
        style={{
          width: size,
          height: size,
          borderRadius: size / 2,
          alignItems: 'center',
          justifyContent: 'center',
          borderWidth: 4,
          borderColor: t.colors.accent,
          boxShadow: t.shadows.raised,
        }}
      >
        <Txt variant="metric" tone="onBrand" style={{ fontSize: size / 2.6, lineHeight: size / 2.3 }}>
          {band === null ? '–' : band.toFixed(1)}
        </Txt>
      </Gradient>
      {label ? (
        <Txt variant="caption" tone="muted">
          {label}
        </Txt>
      ) : null}
    </View>
  );
}

// ---------------------------------------------------------------- EmptyState

export function EmptyState({ icon, title, body, action }: { icon: IconName; title: string; body: string; action?: ReactNode }) {
  const t = useTheme();
  return (
    <View style={{ alignItems: 'center', gap: t.space.md, paddingVertical: t.space.xxl }}>
      <View
        style={{ width: 64, height: 64, borderRadius: 32, backgroundColor: t.colors.primarySoft, alignItems: 'center', justifyContent: 'center' }}
      >
        <Icon name={icon} size={30} color={t.colors.primary} />
      </View>
      <Txt variant="heading" align="center">
        {title}
      </Txt>
      <Txt tone="muted" align="center">
        {body}
      </Txt>
      {action}
    </View>
  );
}

// ---------------------------------------------------------------- Field

interface FieldProps extends TextInputProps {
  label: string;
  hint?: string;
  error?: string | null;
  icon?: IconName;
}

export function Field({ label, hint, error, icon, style, onFocus, onBlur, ...rest }: FieldProps) {
  const t = useTheme();
  const [focused, setFocused] = useState(false);
  const border: ViewStyle = {
    flexDirection: 'row',
    alignItems: 'center',
    gap: t.space.sm,
    minHeight: 56,
    paddingHorizontal: t.space.lg,
    borderRadius: t.radius.md,
    borderWidth: focused ? 2 : 1,
    borderColor: error ? t.colors.danger : focused ? t.colors.primary : t.colors.border,
    backgroundColor: t.colors.surface,
  };
  return (
    <View style={{ gap: t.space.xs }}>
      <Txt variant="label" tone="muted">
        {label}
      </Txt>
      <View style={border}>
        {icon && <Icon name={icon} size={20} color={t.colors.textMuted} />}
        <TextInput
          placeholderTextColor={t.colors.textMuted}
          selectionColor={t.colors.primary}
          onFocus={(e) => {
            setFocused(true);
            onFocus?.(e);
          }}
          onBlur={(e) => {
            setFocused(false);
            onBlur?.(e);
          }}
          style={[{ flex: 1, color: t.colors.text, fontFamily: t.fonts.semibold, fontSize: 18, paddingVertical: t.space.md }, style]}
          {...rest}
        />
      </View>
      {error ? (
        <Txt variant="small" tone="danger">
          {error}
        </Txt>
      ) : hint ? (
        <Txt variant="small" tone="muted">
          {hint}
        </Txt>
      ) : null}
    </View>
  );
}

/** Inline error/notice banner. */
export function Notice({ tone = 'danger', children }: { tone?: 'danger' | 'info' | 'success'; children: ReactNode }) {
  const t = useTheme();
  if (!children) return null;
  const bg = { danger: t.colors.dangerSoft, info: t.colors.infoSoft, success: t.colors.successSoft }[tone];
  const icon: IconName = tone === 'danger' ? 'alert-circle' : tone === 'success' ? 'checkmark-circle' : 'information-circle';
  return (
    <View style={{ flexDirection: 'row', gap: t.space.sm, padding: t.space.md, borderRadius: t.radius.md, backgroundColor: bg }}>
      <Icon name={icon} size={20} color={toneColor(t, tone)} />
      <Txt variant="small" style={{ flex: 1 }}>
        {children}
      </Txt>
    </View>
  );
}
