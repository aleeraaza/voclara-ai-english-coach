import { useState } from 'react';

import { Body, Button, Card, Chips, ErrorText, Screen, Title } from '@/components/ui';
import { api, errorMessage } from '@/lib/api/client';
import { useAuth } from '@/lib/auth';

const LANGUAGES = [
  { value: 'ur', label: 'اردو Urdu' },
  { value: 'hi', label: 'हिन्दी Hindi' },
  { value: 'bn', label: 'বাংলা Bengali' },
  { value: 'ar', label: 'العربية Arabic' },
  { value: 'en', label: 'English' },
] as const;
const LEVELS = ['a1', 'a2', 'b1', 'b2', 'c1', 'c2'].map((v) => ({ value: v, label: v.toUpperCase() }));
const GOALS = [
  { value: 'ielts', label: 'IELTS' },
  { value: 'interview', label: 'Job interviews' },
  { value: 'work', label: 'English at work' },
  { value: 'abroad', label: 'Moving abroad' },
  { value: 'daily', label: 'Daily conversation' },
] as const;

export default function Settings() {
  const { me, refreshMe, signOut } = useAuth();
  const [l1, setL1] = useState(me?.l1 ?? 'ur');
  const [level, setLevel] = useState(me?.level ?? 'b1');
  const [goal, setGoal] = useState(me?.goal ?? 'daily');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setSaving(true);
    setSaved(false);
    setError(null);
    const { error: e } = await api.PATCH('/api/v1/me', { body: { l1, level, goal } });
    setSaving(false);
    if (e) return setError(errorMessage(e));
    await refreshMe();
    setSaved(true);
  }

  return (
    <Screen>
      <Card>
        <Title>Explain corrections in</Title>
        <Chips options={LANGUAGES} value={l1} onChange={setL1} />
      </Card>
      <Card>
        <Title>Your level</Title>
        <Body muted>Sana matches her speed and vocabulary to this.</Body>
        <Chips options={LEVELS} value={level} onChange={setLevel} />
      </Card>
      <Card>
        <Title>Your goal</Title>
        <Chips options={GOALS} value={goal} onChange={setGoal} />
      </Card>
      <Button title={saved ? 'Saved' : 'Save'} onPress={save} loading={saving} />
      <ErrorText>{error}</ErrorText>
      <Button title="Sign out" variant="secondary" onPress={signOut} />
      <Body muted style={{ fontSize: 13 }}>
        Voice recordings are deleted after 30 days and never used to train AI models.
      </Body>
    </Screen>
  );
}
