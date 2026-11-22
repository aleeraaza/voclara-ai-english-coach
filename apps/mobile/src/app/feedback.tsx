import { router } from 'expo-router';

import { Body, Button, Card, ErrorText, Screen, Title, formatMinutes } from '@/components/ui';
import { getLastSession } from '@/lib/lastSession';

const REASON: Record<string, string> = {
  user: 'You ended the session.',
  time_up: 'Time is up for this session.',
  connection_lost: 'The connection dropped, so the session ended.',
  error: 'The session could not continue.',
};

export default function Feedback() {
  const s = getLastSession();
  if (!s) {
    return (
      <Screen>
        <Body muted>No session to show.</Body>
        <Button title="Home" onPress={() => router.replace('/')} />
      </Screen>
    );
  }

  const learnerTurns = s.transcript.filter((t) => t.role === 'learner');
  const words = learnerTurns.reduce((n, t) => n + t.text.split(/\s+/).filter(Boolean).length, 0);

  return (
    <Screen>
      <Card>
        <Title>Session saved</Title>
        <Body muted>{REASON[s.reason] ?? ''}</Body>
        {s.secondsCharged !== null && <Body>Time used: {formatMinutes(s.secondsCharged)}</Body>}
        {s.balanceSecs !== null && <Body>Minutes left: {formatMinutes(s.balanceSecs)}</Body>}
        <Body>
          You spoke {learnerTurns.length} {learnerTurns.length === 1 ? 'time' : 'times'}, about {words} words.
        </Body>
        <ErrorText>{s.liveError}</ErrorText>
        <ErrorText>{s.uploadError}</ErrorText>
      </Card>

      <Card>
        <Title>Feedback</Title>
        <Body muted>
          Grammar fixes, better phrases, filler words and your estimated band will appear here once grading is
          switched on.
        </Body>
      </Card>

      {learnerTurns.length > 0 && (
        <Card>
          <Title>What you said</Title>
          {learnerTurns.map((t, i) => (
            <Body key={i}>• {t.text}</Body>
          ))}
        </Card>
      )}

      <Button title="Back to home" onPress={() => router.replace('/')} />
    </Screen>
  );
}
