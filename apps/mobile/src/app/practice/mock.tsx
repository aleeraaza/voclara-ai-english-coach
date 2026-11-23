import { View } from 'react-native';

import { Card, Gradient, Icon, Notice, Screen, SectionTitle, Txt, type IconName } from '@/components';
import { StartPanel } from '@/components/StartPanel';
import { useTheme } from '@/theme';

const PARTS: { icon: IconName; title: string; time: string; body: string }[] = [
  { icon: 'person', title: 'Part 1 · Introduction', time: '4–5 min', body: 'Short questions about you, your home, work or studies.' },
  { icon: 'timer', title: 'Part 2 · Long turn', time: '3–4 min', body: 'A cue card on your screen: one minute to prepare, then speak for up to two minutes.' },
  { icon: 'chatbubbles', title: 'Part 3 · Discussion', time: '4–5 min', body: 'Deeper questions linked to your Part 2 topic.' },
];

export default function MockSetup() {
  const t = useTheme();
  return (
    <Screen>
      <Gradient name="mock" style={{ borderRadius: t.radius.xl, padding: t.space.xl, gap: t.space.xs }}>
        <Txt variant="label" tone="onAccentMuted">
          About 15 minutes
        </Txt>
        <Txt variant="title" tone="onAccent">
          IELTS-style speaking test
        </Txt>
        <Txt variant="small" tone="onAccentMuted">
          A calm examiner runs all three parts. Your estimated band arrives right after.
        </Txt>
      </Gradient>

      <SectionTitle>What to expect</SectionTitle>
      <Card style={{ gap: t.space.lg }}>
        {PARTS.map((p, i) => (
          <View key={p.title} style={{ flexDirection: 'row', gap: t.space.md }}>
            <View style={{ alignItems: 'center' }}>
              <View
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: 20,
                  backgroundColor: t.colors.accentSoft,
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Icon name={p.icon} size={20} color={t.colors.accent} />
              </View>
              {i < PARTS.length - 1 && <View style={{ width: 2, flex: 1, minHeight: 20, backgroundColor: t.colors.border, marginTop: 4 }} />}
            </View>
            <View style={{ flex: 1, gap: 2, paddingBottom: t.space.xs }}>
              <View style={{ flexDirection: 'row', justifyContent: 'space-between' }}>
                <Txt variant="bodyStrong">{p.title}</Txt>
                <Txt variant="caption" tone="muted">
                  {p.time}
                </Txt>
              </View>
              <Txt variant="small" tone="muted">
                {p.body}
              </Txt>
            </View>
          </View>
        ))}
      </Card>

      <Notice tone="info">
        Find a quiet place. Each test uses a new topic from 30+ cue cards. The examiner does not correct you during the test;
        your band and top 3 fixes come after.
      </Notice>

      <StartPanel mode="mock" limitMins={15} />
    </Screen>
  );
}
