import { useState } from 'react';
import { View } from 'react-native';

import { ChipGroup, Field, Gradient, Screen, SectionTitle, Txt, VoiceOrb } from '@/components';
import { StartPanel } from '@/components/StartPanel';
import { useTheme } from '@/theme';

const TOPICS = [
  { value: 'my day', label: 'My day', icon: 'sunny' },
  { value: 'work and studies', label: 'Work & study', icon: 'school' },
  { value: 'my hometown', label: 'Hometown', icon: 'home' },
  { value: 'cricket and sports', label: 'Cricket', icon: 'trophy' },
  { value: 'food and cooking', label: 'Food', icon: 'restaurant' },
  { value: 'travel', label: 'Travel', icon: 'airplane' },
  { value: 'technology', label: 'Technology', icon: 'phone-portrait' },
  { value: 'future plans', label: 'Future plans', icon: 'rocket' },
] as const;

export default function TalkSetup() {
  const t = useTheme();
  const [picked, setPicked] = useState<string>('my day');
  const [custom, setCustom] = useState('');
  const topic = custom.trim() || picked;

  return (
    <Screen>
      <Gradient name="brand" style={{ borderRadius: t.radius.xl, padding: t.space.xl, flexDirection: 'row', alignItems: 'center' }}>
        <View style={{ flex: 1, gap: t.space.xs }}>
          <Txt variant="heading" tone="onBrand">
            A relaxed chat with Sana
          </Txt>
          <Txt variant="small" tone="onBrandMuted">
            She asks questions, follows up on your answers and gently fixes one mistake at a time.
          </Txt>
        </View>
        <VoiceOrb size={56} state="idle" />
      </Gradient>

      <SectionTitle>Pick a topic</SectionTitle>
      <ChipGroup
        options={TOPICS}
        value={custom.trim() ? null : picked}
        onChange={(v) => {
          setPicked(v);
          setCustom('');
        }}
      />
      <Field
        label="Or type your own"
        icon="create-outline"
        placeholder="e.g. my first job interview"
        value={custom}
        onChangeText={setCustom}
        maxLength={120}
      />

      <StartPanel mode="talk" topic={topic} limitMins={10} />
    </Screen>
  );
}
