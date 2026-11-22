import { useFocusEffect, router } from 'expo-router';
import { useCallback, useState } from 'react';
import { View } from 'react-native';

import { Body, Button, Card, Chips, Field, Screen, Title, formatMinutes } from '@/components/ui';
import { useAuth } from '@/lib/auth';

const ROLES = [
  { value: 'software', label: 'Software' },
  { value: 'sales', label: 'Sales' },
  { value: 'banking', label: 'Banking' },
  { value: 'nursing', label: 'Nursing' },
  { value: 'call_centre', label: 'Call centre' },
] as const;
type Role = (typeof ROLES)[number]['value'];

export default function Home() {
  const { me, refreshMe } = useAuth();
  const [topic, setTopic] = useState('');
  const [role, setRole] = useState<Role>('software');

  // Minutes change after every session; reload whenever home is shown.
  useFocusEffect(
    useCallback(() => {
      refreshMe().catch(() => null);
    }, [refreshMe]),
  );

  const balance = me?.minutes.balance_secs ?? 0;
  const lowOnMinutes = balance < 30;

  const go = (mode: 'talk' | 'mock' | 'interview', t?: string) =>
    router.push({ pathname: '/talk', params: t ? { mode, topic: t } : { mode } });

  return (
    <Screen>
      <View style={{ gap: 4 }}>
        <Title>{me?.name ? `Hi ${me.name}` : 'Ready to speak?'}</Title>
        <Body muted>
          {formatMinutes(balance)} of live practice left · {me?.plan === 'free' ? 'Free plan' : me?.plan ?? ''}
        </Body>
      </View>

      {lowOnMinutes && (
        <Card>
          <Body>You are out of live minutes. Plans and top-ups are coming soon.</Body>
        </Card>
      )}

      <Card>
        <Title>Talk with Sana</Title>
        <Body muted>A relaxed conversation. Sana corrects one important mistake at a time.</Body>
        <Field label="Topic (optional)" placeholder="e.g. my job, cricket, travel" value={topic} onChangeText={setTopic} />
        <Button title="Start talking" disabled={lowOnMinutes} onPress={() => go('talk', topic.trim() || undefined)} />
      </Card>

      <Card>
        <Title>IELTS-style speaking mock</Title>
        <Body muted>Parts 1, 2 and 3 with an examiner. Band estimate after the test.</Body>
        <Button title="Start mock test" disabled={lowOnMinutes} onPress={() => go('mock')} />
      </Card>

      <Card>
        <Title>Job interview</Title>
        <Body muted>Pick a role. The interviewer asks follow-up questions.</Body>
        <Chips options={ROLES} value={role} onChange={setRole} />
        <Button title="Start interview" disabled={lowOnMinutes} onPress={() => go('interview', role)} />
      </Card>

      <View style={{ flexDirection: 'row', gap: 12 }}>
        <Button title="Progress" variant="secondary" style={{ flex: 1 }} onPress={() => router.push('/progress')} />
        <Button title="Settings" variant="secondary" style={{ flex: 1 }} onPress={() => router.push('/settings')} />
      </View>
    </Screen>
  );
}
