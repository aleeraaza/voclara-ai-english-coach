import { useFocusEffect } from 'expo-router';
import { useCallback, useState } from 'react';
import { StyleSheet, View } from 'react-native';

import { Body, Card, Screen, Title, formatMinutes, useTheme } from '@/components/ui';
import { api, type Progress as ProgressData } from '@/lib/api/client';
import { useAuth } from '@/lib/auth';

const MODE: Record<string, string> = { talk: 'Talk', mock: 'Mock test', interview: 'Interview' };

export default function Progress() {
  const t = useTheme();
  const { me, refreshMe } = useAuth();
  const [progress, setProgress] = useState<ProgressData | null>(null);

  useFocusEffect(
    useCallback(() => {
      refreshMe().catch(() => null);
      api
        .GET('/api/v1/progress')
        .then(({ data }) => data && setProgress(data))
        .catch(() => null);
    }, [refreshMe]),
  );

  const recent = progress?.recent ?? [];

  return (
    <Screen>
      <Card>
        <Title>{me?.streak.current_days ?? 0}-day streak</Title>
        <Body muted>Best: {me?.streak.best_days ?? 0} days</Body>
      </Card>

      <Card>
        <Title>Band score trend</Title>
        {progress?.average_overall_last5 !== null && progress?.average_overall_last5 !== undefined ? (
          <Body>
            Average of your last {Math.min(5, progress.graded_sessions)} graded sessions:{' '}
            <Body style={{ fontWeight: '800' }}>{progress.average_overall_last5.toFixed(1)}</Body>
          </Body>
        ) : (
          <Body muted>Finish a session of a few minutes to get your first estimated band.</Body>
        )}
        {recent.map((r) => (
          <View key={r.session_id} style={styles.item}>
            <View style={styles.row}>
              <Body style={{ flex: 1 }}>
                {MODE[r.mode] ?? r.mode} · {new Date(r.started_at).toLocaleDateString()}
                {r.demo ? ' · demo' : ''}
              </Body>
              <Body style={{ fontWeight: '800' }}>{r.overall.toFixed(1)}</Body>
            </View>
            <View style={[styles.track, { backgroundColor: t.border }]}>
              <View
                style={[
                  styles.fill,
                  { backgroundColor: r.demo ? t.muted : t.primary, width: `${(r.overall / 9) * 100}%` },
                ]}
              />
            </View>
          </View>
        ))}
      </Card>

      <Card>
        <Title>Minutes</Title>
        <Body>Live practice left: {formatMinutes(me?.minutes.balance_secs ?? 0)}</Body>
        <Body>Free drills left today: {formatMinutes(me?.minutes.daily_drill_secs_left ?? 0)}</Body>
      </Card>
    </Screen>
  );
}

const styles = StyleSheet.create({
  item: { gap: 6, paddingVertical: 4 },
  row: { flexDirection: 'row', alignItems: 'center' },
  track: { height: 8, borderRadius: 4, overflow: 'hidden' },
  fill: { height: 8, borderRadius: 4 },
});
