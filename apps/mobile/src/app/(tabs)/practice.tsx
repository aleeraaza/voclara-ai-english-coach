import { router } from 'expo-router';
import { View } from 'react-native';

import { Card, Icon, ModeTile, Screen, SectionTitle, Txt, type IconName } from '@/components';
import { useTheme } from '@/theme';

const STEPS: { icon: IconName; title: string; body: string }[] = [
  { icon: 'options', title: 'Pick a mode', body: 'Chat, a mock test or an interview.' },
  { icon: 'mic', title: 'Speak naturally', body: 'Hold the button, or go hands-free.' },
  { icon: 'ribbon', title: 'Get your band', body: 'Bands, fixes and tips in a minute.' },
];

export default function Practice() {
  const t = useTheme();
  return (
    <Screen edges={['top', 'left', 'right']} title="Practice" subtitle="Choose how you want to speak today." wash>
      <ModeTile
        gradient="talk"
        icon="chatbubbles"
        title="Talk with Sana"
        subtitle="Relaxed conversation on a topic you choose"
        badge="10 min"
        onPress={() => router.push('/practice/talk')}
      />
      <ModeTile
        gradient="mock"
        icon="document-text"
        title="IELTS-style mock test"
        subtitle="Parts 1, 2 and 3 with an examiner, then your band"
        badge="15 min"
        onPress={() => router.push('/practice/mock')}
      />
      <ModeTile
        gradient="interview"
        icon="briefcase"
        title="Job interview"
        subtitle="Software, sales, banking, nursing or call centre"
        badge="15 min"
        onPress={() => router.push('/practice/interview')}
      />
      <ModeTile
        gradient="drill"
        icon="musical-notes"
        title="Pronunciation drills"
        subtitle="v / w, th, short vowels and more"
        badge="Soon"
        disabled
      />

      <SectionTitle>How it works</SectionTitle>
      <Card>
        {STEPS.map((s, i) => (
          <View key={s.title} style={{ flexDirection: 'row', gap: t.space.md, alignItems: 'center' }}>
            <View
              style={{
                width: 40,
                height: 40,
                borderRadius: 20,
                backgroundColor: t.colors.primarySoft,
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <Icon name={s.icon} size={20} color={t.colors.primary} />
            </View>
            <View style={{ flex: 1 }}>
              <Txt variant="bodyStrong">
                {i + 1}. {s.title}
              </Txt>
              <Txt variant="small" tone="muted">
                {s.body}
              </Txt>
            </View>
          </View>
        ))}
      </Card>
    </Screen>
  );
}
