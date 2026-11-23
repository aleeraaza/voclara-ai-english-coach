/**
 * Animated launch screen, shown over the app while fonts and the session
 * load. Stays at least `motion.splashMin` so it never flickers, then fades.
 *
 * (The very first native splash is configured in app.json and changes only
 * with a new build; this screen takes over as soon as JavaScript runs.)
 */
import { useEffect, useState } from 'react';
import { Animated, Easing, StyleSheet, View } from 'react-native';
import { useAnimatedValue } from './anim';

import { useTheme } from '@/theme';

import { Gradient, Txt } from './core';
import { SoundWave, VoiceOrb } from './voice';

interface SplashProps {
  /** Fonts loaded and auth state known. */
  ready: boolean;
  /** Fonts are usable (the wordmark waits for them). */
  fontsReady: boolean;
  onDone: () => void;
}

export function Splash({ ready, fontsReady, onDone }: SplashProps) {
  const t = useTheme();
  const orb = useAnimatedValue(0);
  const words = useAnimatedValue(0);
  const tagline = useAnimatedValue(0);
  const exit = useAnimatedValue(1);
  const [minElapsed, setMinElapsed] = useState(false);

  useEffect(() => {
    Animated.spring(orb, { toValue: 1, useNativeDriver: true, speed: 6, bounciness: 8 }).start();
    const id = setTimeout(() => setMinElapsed(true), t.motion.splashMin);
    return () => clearTimeout(id);
  }, [orb, t.motion.splashMin]);

  useEffect(() => {
    if (!fontsReady) return;
    Animated.stagger(180, [
      Animated.timing(words, { toValue: 1, duration: t.motion.intro, easing: Easing.out(Easing.cubic), useNativeDriver: true }),
      Animated.timing(tagline, { toValue: 1, duration: t.motion.intro, easing: Easing.out(Easing.cubic), useNativeDriver: true }),
    ]).start();
  }, [fontsReady, tagline, words, t.motion.intro]);

  useEffect(() => {
    if (!ready || !minElapsed) return;
    Animated.timing(exit, { toValue: 0, duration: t.motion.slow, easing: Easing.in(Easing.quad), useNativeDriver: true }).start(
      ({ finished }) => finished && onDone(),
    );
  }, [exit, minElapsed, onDone, ready, t.motion.slow]);

  const rise = (v: Animated.Value) => ({
    opacity: v,
    transform: [{ translateY: v.interpolate({ inputRange: [0, 1], outputRange: [16, 0] }) }],
  });

  return (
    <Animated.View style={[StyleSheet.absoluteFill, { opacity: exit, zIndex: 100 }]} accessibilityLabel="Voclara is loading">
      <Gradient name="brand" style={styles.fill}>
        <Animated.View
          style={{
            opacity: orb,
            transform: [{ scale: orb.interpolate({ inputRange: [0, 1], outputRange: [0.6, 1] }) }],
          }}
        >
          <VoiceOrb size={140} state="speaking" icon="mic" />
        </Animated.View>
        <View style={{ height: t.space.lg }} />
        <SoundWave bars={7} height={34} color={t.colors.accent} />
        <View style={{ height: t.space.xxl }} />
        {fontsReady && (
          <>
            <Animated.View style={rise(words)}>
              <Txt variant="display" tone="onBrand" style={{ fontSize: 44, lineHeight: 50 }}>
                Voclara
              </Txt>
            </Animated.View>
            <Animated.View style={rise(tagline)}>
              <Txt tone="onBrandMuted" align="center">
                Speak English with confidence
              </Txt>
            </Animated.View>
          </>
        )}
      </Gradient>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  fill: { flex: 1, alignItems: 'center', justifyContent: 'center', padding: 24 },
});
