import Constants from 'expo-constants';
import { View } from 'react-native';

import { Card, Gradient, Icon, Screen, Txt, type IconName } from '@/components';
import { useTheme } from '@/theme';

const POINTS: { icon: IconName; title: string; body: string }[] = [
  { icon: 'mic', title: 'Your voice', body: 'Used only to run your session and give feedback. Recordings are deleted after 30 days.' },
  { icon: 'ban', title: 'No AI training', body: 'We never use your audio or transcripts to train AI models.' },
  { icon: 'trash', title: 'Delete anytime', body: 'Deleting your account removes your sessions, scores and recordings.' },
  { icon: 'information-circle', title: 'Estimated bands', body: 'Bands are IELTS-style estimates for practice. Voclara is not affiliated with IELTS.' },
];

export default function About() {
  const t = useTheme();
  return (
    <Screen>
      <View style={{ alignItems: 'center', gap: t.space.sm, paddingVertical: t.space.lg }}>
        <Gradient name="brand" style={{ width: 72, height: 72, borderRadius: 36, alignItems: 'center', justifyContent: 'center' }}>
          <Icon name="mic" size={34} color={t.colors.onBrand} />
        </Gradient>
        <Txt variant="title">Voclara</Txt>
        <Txt tone="muted">Version {Constants.expoConfig?.version ?? '0.1.0'}</Txt>
      </View>
      <Card>
        {POINTS.map((p) => (
          <View key={p.title} style={{ flexDirection: 'row', gap: t.space.md }}>
            <Icon name={p.icon} color={t.colors.primary} />
            <View style={{ flex: 1, gap: 2 }}>
              <Txt variant="bodyStrong">{p.title}</Txt>
              <Txt variant="small" tone="muted">
                {p.body}
              </Txt>
            </View>
          </View>
        ))}
      </Card>
    </Screen>
  );
}
