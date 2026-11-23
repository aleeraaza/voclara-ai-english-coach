/**
 * The tutor's visual presence: an animated orb and sound-wave bars.
 * Built from tokens (gradients + Animated), so no images are needed.
 */
import { useEffect, useState } from 'react';
import { Animated, Easing, View, type StyleProp, type ViewStyle } from 'react-native';
import { useAnimatedValue } from './anim';

import { useTheme } from '@/theme';

import { Icon, type IconName } from './core';

export type OrbState = 'idle' | 'listening' | 'speaking';

interface VoiceOrbProps {
  size?: number;
  state?: OrbState;
  /** Mic level 0..1 while listening; grows the orb a little. */
  level?: number;
  icon?: IconName;
  style?: StyleProp<ViewStyle>;
}

export function VoiceOrb({ size = 180, state = 'idle', level = 0, icon, style }: VoiceOrbProps) {
  const t = useTheme();
  const breath = useAnimatedValue(0);
  const ring = useAnimatedValue(0);
  const levelAnim = useAnimatedValue(0);

  // Idle breathing; faster while the tutor speaks.
  useEffect(() => {
    const duration = state === 'speaking' ? t.motion.breath / 2 : t.motion.breath;
    const loop = Animated.loop(
      Animated.sequence([
        Animated.timing(breath, { toValue: 1, duration, easing: Easing.inOut(Easing.sin), useNativeDriver: true }),
        Animated.timing(breath, { toValue: 0, duration, easing: Easing.inOut(Easing.sin), useNativeDriver: true }),
      ]),
    );
    loop.start();
    return () => loop.stop();
  }, [breath, state, t.motion.breath]);

  // Expanding rings while someone is talking.
  useEffect(() => {
    if (state === 'idle') {
      ring.setValue(0);
      return;
    }
    const loop = Animated.loop(
      Animated.timing(ring, { toValue: 1, duration: t.motion.breath, easing: Easing.out(Easing.quad), useNativeDriver: true }),
    );
    loop.start();
    return () => loop.stop();
  }, [ring, state, t.motion.breath]);

  useEffect(() => {
    Animated.spring(levelAnim, { toValue: Math.min(1, level * 4), useNativeDriver: true, speed: 20, bounciness: 4 }).start();
  }, [level, levelAnim]);

  const coreScale = Animated.add(
    breath.interpolate({ inputRange: [0, 1], outputRange: [0.96, 1.04] }),
    levelAnim.interpolate({ inputRange: [0, 1], outputRange: [0, 0.12] }),
  );
  const ringScale = ring.interpolate({ inputRange: [0, 1], outputRange: [1, 1.55] });
  const ringOpacity = ring.interpolate({ inputRange: [0, 1], outputRange: [0.45, 0] });
  const ringColor = state === 'speaking' ? t.colors.accent : t.colors.primary;

  return (
    <View
      style={[{ width: size * 1.6, height: size * 1.6, alignItems: 'center', justifyContent: 'center' }, style]}
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
    >
      <Animated.View
        style={{
          position: 'absolute',
          width: size,
          height: size,
          borderRadius: size / 2,
          borderWidth: 2,
          borderColor: ringColor,
          opacity: ringOpacity,
          transform: [{ scale: ringScale }],
        }}
      />
      <Animated.View
        style={{
          width: size,
          height: size,
          borderRadius: size / 2,
          backgroundImage: t.gradients.orb,
          boxShadow: `0px 0px ${Math.round(size / 3)}px ${t.colors.primary}66`,
          alignItems: 'center',
          justifyContent: 'center',
          transform: [{ scale: coreScale }],
        }}
      >
        {icon && <Icon name={icon} size={size / 3.2} color={t.colors.onBrand} />}
      </Animated.View>
    </View>
  );
}

interface SoundWaveProps {
  bars?: number;
  height?: number;
  active?: boolean;
  color?: string;
}

/** Bouncing bars: "voice" at a glance (splash, listening indicators). */
export function SoundWave({ bars = 5, height = 36, active = true, color }: SoundWaveProps) {
  const t = useTheme();
  const [values] = useState(() => Array.from({ length: bars }, () => new Animated.Value(0.3)));

  useEffect(() => {
    if (!active) {
      values.forEach((v) => v.setValue(0.3));
      return;
    }
    const loops = values.map((v, i) =>
      Animated.loop(
        Animated.sequence([
          Animated.delay(i * 110),
          Animated.timing(v, { toValue: 1, duration: 420, easing: Easing.inOut(Easing.quad), useNativeDriver: true }),
          Animated.timing(v, { toValue: 0.3, duration: 420, easing: Easing.inOut(Easing.quad), useNativeDriver: true }),
        ]),
      ),
    );
    loops.forEach((l) => l.start());
    return () => loops.forEach((l) => l.stop());
  }, [active, values]);

  return (
    <View style={{ flexDirection: 'row', alignItems: 'center', gap: 5, height }} accessibilityElementsHidden>
      {values.map((v, i) => (
        <Animated.View
          key={i}
          style={{
            width: 6,
            height,
            borderRadius: 3,
            backgroundColor: color ?? t.colors.accent,
            transform: [{ scaleY: v }],
          }}
        />
      ))}
    </View>
  );
}
