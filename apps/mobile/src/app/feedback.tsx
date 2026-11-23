import { router } from 'expo-router';
import { View } from 'react-native';

import { Button, Card, EmptyState, Icon, Notice, Screen, SectionTitle, Txt } from '@/components';
import { FeedbackView, useFeedback } from '@/components/FeedbackView';
import { formatMinutes } from '@/lib/format';
import { getLastSession } from '@/lib/lastSession';
import { useTheme } from '@/theme';

const REASON: Record<string, string> = {
  user: 'Session finished',
  time_up: 'Time is up',
  connection_lost: 'The connection dropped',
  error: 'The session stopped early',
};

export default function AfterSession() {
  const t = useTheme();
  const s = getLastSession();
  const { fb, error } = useFeedback(s?.sessionId ?? null, s?.secondsCharged !== null);

  if (!s) {
    return (
      <Screen>
        <EmptyState icon="chatbubbles" title="No session to show" body="Start a practice session from Home." />
        <Button title="Go home" onPress={() => router.replace('/')} />
      </Screen>
    );
  }

  const learnerTurns = s.transcript.filter((x) => x.role === 'learner');
  const words = learnerTurns.reduce((n, x) => n + x.text.split(/\s+/).filter(Boolean).length, 0);

  return (
    <Screen>
      <View style={{ flexDirection: 'row', gap: t.space.md }}>
        <Card style={{ flex: 1, gap: t.space.xs }}>
          <Icon name="checkmark-done" color={t.colors.success} />
          <Txt variant="bodyStrong">{REASON[s.reason] ?? 'Saved'}</Txt>
          <Txt variant="caption" tone="muted">
            {s.secondsCharged !== null ? `${formatMinutes(s.secondsCharged)} used` : 'Not saved'}
          </Txt>
        </Card>
        <Card style={{ flex: 1, gap: t.space.xs }}>
          <Icon name="mic" color={t.colors.primary} />
          <Txt variant="bodyStrong">{words} words</Txt>
          <Txt variant="caption" tone="muted">
            {s.balanceSecs !== null ? `${formatMinutes(s.balanceSecs)} left` : `${learnerTurns.length} turns`}
          </Txt>
        </Card>
      </View>

      <Notice>{s.liveError}</Notice>
      <Notice>{s.uploadError}</Notice>

      {s.secondsCharged !== null && <FeedbackView fb={fb} error={error} />}

      {learnerTurns.length > 0 && (
        <>
          <SectionTitle>What you said</SectionTitle>
          <Card style={{ gap: t.space.sm }}>
            {learnerTurns.map((x, i) => (
              <Txt key={i} variant="small">
                • {x.text}
              </Txt>
            ))}
          </Card>
        </>
      )}

      <Button title="Practise again" variant="cta" icon="mic" size="lg" onPress={() => router.replace('/practice')} />
      <Button title="Back to home" variant="ghost" onPress={() => router.replace('/')} />
    </Screen>
  );
}
