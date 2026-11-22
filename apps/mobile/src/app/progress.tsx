import { useFocusEffect } from 'expo-router';
import { useCallback } from 'react';

import { Body, Card, Screen, Title, formatMinutes } from '@/components/ui';
import { useAuth } from '@/lib/auth';

export default function Progress() {
  const { me, refreshMe } = useAuth();
  useFocusEffect(
    useCallback(() => {
      refreshMe().catch(() => null);
    }, [refreshMe]),
  );

  return (
    <Screen>
      <Card>
        <Title>{me?.streak.current_days ?? 0}-day streak</Title>
        <Body muted>Best: {me?.streak.best_days ?? 0} days</Body>
      </Card>
      <Card>
        <Title>Minutes</Title>
        <Body>Live practice left: {formatMinutes(me?.minutes.balance_secs ?? 0)}</Body>
        <Body>Free drills left today: {formatMinutes(me?.minutes.daily_drill_secs_left ?? 0)}</Body>
      </Card>
      <Card>
        <Title>Band score trend</Title>
        <Body muted>Your estimated bands will appear here once session grading is switched on.</Body>
      </Card>
    </Screen>
  );
}
