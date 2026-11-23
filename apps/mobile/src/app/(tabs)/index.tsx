import { router, useFocusEffect } from 'expo-router';
import { useCallback } from 'react';
import { Pressable, View } from 'react-native';

import {
  Button,
  Card,
  Gradient,
  Icon,
  ModeTile,
  Notice,
  Screen,
  SectionTitle,
  StatWidget,
  Txt,
  VoiceOrb,
} from '@/components';
import { useAuth } from '@/lib/auth';
import { formatMinutes, greeting } from '@/lib/format';
import { useProgress } from '@/lib/useProgress';
import { useTheme } from '@/theme';

const TIPS = [
  'Answer in full sentences. "Yes" is easy; "Yes, because…" shows real fluency.',
  'Pausing is fine. Say "Let me think…" instead of "umm".',
  'Use past tense for anything that already happened: "yesterday I went", not "I go".',
  'Give one example for every opinion. Examiners love specifics.',
  'Record a 2-minute talk daily. Small, regular practice beats one long session.',
  'Learn phrases, not single words: "make a decision", "take a break".',
  'Speak a little slower than feels natural. Clear beats fast.',
];

function tipOfTheDay(): string {
  const day = Math.floor(Date.now() / 86_400_000);
  return TIPS[day % TIPS.length]!;
}

export default function Home() {
  const t = useTheme();
  const { me, refreshMe } = useAuth();
  const progress = useProgress();

  useFocusEffect(
    useCallback(() => {
      refreshMe().catch(() => null);
    }, [refreshMe]),
  );

  const balance = me?.minutes.balance_secs ?? 0;
  const lowOnMinutes = me !== null && balance < 30;
  const firstName = me?.name?.split(' ')[0];
  const latest = progress?.recent.find((r) => !r.demo) ?? progress?.recent[0];
  const initial = (me?.name ?? 'V').trim().charAt(0).toUpperCase();

  return (
    <Screen edges={['top', 'left', 'right']} wash>
      {/* Header */}
      <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.md }}>
        <View style={{ flex: 1 }}>
          <Txt tone="muted">{greeting()}</Txt>
          <Txt variant="title" accessibilityRole="header">
            {firstName ? `${firstName} 👋` : 'Ready to speak? 👋'}
          </Txt>
        </View>
        <Pressable accessibilityRole="button" accessibilityLabel="Open your profile" onPress={() => router.navigate('/profile')}>
          <Gradient name="brand" style={{ width: 48, height: 48, borderRadius: 24, alignItems: 'center', justifyContent: 'center' }}>
            <Txt variant="heading" tone="onBrand">
              {initial}
            </Txt>
          </Gradient>
        </Pressable>
      </View>

      {/* Hero */}
      <Gradient name="brand" style={{ borderRadius: t.radius.xl, padding: t.space.xl, boxShadow: t.shadows.raised }}>
        <View style={{ flexDirection: 'row', alignItems: 'center' }}>
          <View style={{ flex: 1, gap: t.space.xs }}>
            <Txt variant="label" tone="onBrandMuted">
              Daily conversation
            </Txt>
            <Txt variant="title" tone="onBrand">
              Talk with Sana
            </Txt>
            <Txt variant="small" tone="onBrandMuted">
              A friendly 10-minute chat on any topic. She fixes one mistake at a time.
            </Txt>
          </View>
          <VoiceOrb size={64} state="idle" icon="mic" style={{ marginRight: -t.space.md }} />
        </View>
        <Button
          title="Start talking"
          variant="cta"
          icon="mic"
          disabled={lowOnMinutes}
          onPress={() => router.push('/practice/talk')}
          style={{ marginTop: t.space.lg, alignSelf: 'flex-start' }}
        />
      </Gradient>

      {lowOnMinutes && <Notice tone="info">You have used your free minutes. Plans and top-ups are coming soon.</Notice>}

      {/* Widgets */}
      <View style={{ flexDirection: 'row', gap: t.space.md }}>
        <StatWidget icon="time" value={formatMinutes(balance)} label="Practice left" tone="primary" onPress={() => router.navigate('/profile')} />
        <StatWidget icon="flame" value={`${me?.streak.current_days ?? 0}`} label="Day streak" tone="accent" onPress={() => router.navigate('/progress')} />
        <StatWidget
          icon="ribbon"
          value={latest ? latest.overall.toFixed(1) : '–'}
          label="Latest band"
          tone="info"
          onPress={() => router.navigate('/progress')}
        />
      </View>

      {/* Quick practice */}
      <SectionTitle
        action={
          <Pressable accessibilityRole="button" onPress={() => router.navigate('/practice')} hitSlop={8}>
            <Txt variant="small" tone="primary" style={{ fontFamily: t.fonts.bold }}>
              See all
            </Txt>
          </Pressable>
        }
      >
        Practise for a goal
      </SectionTitle>
      <View style={{ flexDirection: 'row', gap: t.space.md }}>
        <ModeTile
          compact
          gradient="mock"
          icon="document-text"
          title="Mock test"
          subtitle="IELTS-style, Parts 1–3"
          onPress={() => router.push('/practice/mock')}
        />
        <ModeTile
          compact
          gradient="interview"
          icon="briefcase"
          title="Interview"
          subtitle="Pick a role, get grilled"
          onPress={() => router.push('/practice/interview')}
        />
      </View>

      {/* Tip */}
      <Card tone="soft" style={{ flexDirection: 'row', gap: t.space.md, alignItems: 'flex-start' }}>
        <View style={{ width: 40, height: 40, borderRadius: 20, backgroundColor: t.colors.surface, alignItems: 'center', justifyContent: 'center' }}>
          <Icon name="bulb" color={t.colors.accent} />
        </View>
        <View style={{ flex: 1, gap: t.space.xxs }}>
          <Txt variant="label" tone="primary">
            Tip of the day
          </Txt>
          <Txt variant="small">{tipOfTheDay()}</Txt>
        </View>
      </Card>
    </Screen>
  );
}
