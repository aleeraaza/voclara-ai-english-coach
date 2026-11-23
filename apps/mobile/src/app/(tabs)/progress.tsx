import { router, useFocusEffect } from 'expo-router';
import { useCallback } from 'react';
import { View } from 'react-native';

import { Button, Card, EmptyState, ListGroup, ListRow, Screen, SectionTitle, StatWidget, Txt } from '@/components';
import { BandChart } from '@/components/BandChart';
import { useAuth } from '@/lib/auth';
import { useProgress } from '@/lib/useProgress';
import { useTheme } from '@/theme';

const MODE: Record<string, { label: string; icon: 'chatbubbles' | 'document-text' | 'briefcase' }> = {
  talk: { label: 'Conversation', icon: 'chatbubbles' },
  mock: { label: 'Mock test', icon: 'document-text' },
  interview: { label: 'Interview', icon: 'briefcase' },
};

function shortDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
}

export default function Progress() {
  const t = useTheme();
  const { me, refreshMe } = useAuth();
  const progress = useProgress();
  useFocusEffect(
    useCallback(() => {
      refreshMe().catch(() => null);
    }, [refreshMe]),
  );

  const recent = progress?.recent ?? [];
  const avg = progress?.average_overall_last5;

  return (
    <Screen edges={['top', 'left', 'right']} title="Progress" subtitle="Your estimated bands over time." wash>
      <View style={{ flexDirection: 'row', gap: t.space.md }}>
        <StatWidget icon="ribbon" value={avg != null ? avg.toFixed(1) : '–'} label="Avg of last 5" tone="info" />
        <StatWidget icon="checkmark-done" value={`${progress?.graded_sessions ?? 0}`} label="Graded" tone="primary" />
        <StatWidget icon="flame" value={`${me?.streak.best_days ?? 0}`} label="Best streak" tone="accent" />
      </View>

      {recent.length === 0 ? (
        <Card>
          <EmptyState
            icon="stats-chart"
            title="No bands yet"
            body="Finish a session of a few minutes and your first estimated band appears here."
            action={<Button title="Start practising" icon="mic" onPress={() => router.navigate('/practice')} />}
          />
        </Card>
      ) : (
        <>
          <Card>
            <Txt variant="heading">Band trend</Txt>
            <BandChart
              points={[...recent].reverse().map((r) => ({
                id: r.session_id,
                band: r.overall,
                label: shortDate(r.started_at),
                demo: r.demo,
              }))}
            />
            {recent.some((r) => r.demo) && (
              <Txt variant="caption" tone="muted">
                Grey bars are demo grades and do not count toward your average.
              </Txt>
            )}
          </Card>

          <SectionTitle>History</SectionTitle>
          <ListGroup>
            {recent.map((r) => (
              <ListRow
                key={r.session_id}
                icon={MODE[r.mode]?.icon ?? 'chatbubbles'}
                title={`${MODE[r.mode]?.label ?? r.mode}${r.demo ? ' · demo' : ''}`}
                subtitle={shortDate(r.started_at)}
                value={r.overall.toFixed(1)}
                onPress={() => router.push({ pathname: '/session/[id]', params: { id: r.session_id } })}
              />
            ))}
          </ListGroup>
        </>
      )}
    </Screen>
  );
}
