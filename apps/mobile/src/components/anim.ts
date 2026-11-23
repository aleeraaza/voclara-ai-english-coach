import { useState } from 'react';
import { Animated } from 'react-native';

/**
 * A stable Animated.Value for the component's lifetime.
 * Lazy state instead of `useRef(...).current` (the React Compiler lint forbids
 * reading refs during render) and instead of React Native's own
 * `useAnimatedValue`, which react-native-web does not ship.
 */
export function useAnimatedValue(initial: number): Animated.Value {
  const [value] = useState(() => new Animated.Value(initial));
  return value;
}
