import { useRef, useState, type ComponentRef } from 'react';
import { ScrollView, StyleSheet, View, useWindowDimensions } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { BandBadge, Button, Gradient, Icon, ProgressBar, Txt, VoiceOrb } from '@/components';
import { useOnboarding } from '@/lib/auth';
import { useTheme } from '@/theme';

function Bubble({ text, align, tone }: { text: string; align: 'left' | 'right'; tone: 'tutor' | 'learner' }) {
  const t = useTheme();
  return (
    <View
      style={{
        alignSelf: align === 'left' ? 'flex-start' : 'flex-end',
        maxWidth: '78%',
        backgroundColor: tone === 'learner' ? t.colors.primary : t.colors.surface,
        borderRadius: t.radius.lg,
        borderBottomLeftRadius: align === 'left' ? 4 : t.radius.lg,
        borderBottomRightRadius: align === 'right' ? 4 : t.radius.lg,
        paddingHorizontal: t.space.lg,
        paddingVertical: t.space.md,
        boxShadow: t.shadows.raised,
      }}
    >
      <Txt variant="small" tone={tone === 'learner' ? 'onPrimary' : 'default'}>
        {text}
      </Txt>
    </View>
  );
}

function ArtTalk() {
  const t = useTheme();
  return (
    <View style={{ alignItems: 'center', gap: t.space.md, width: '100%' }}>
      <VoiceOrb size={130} state="speaking" icon="mic" />
      <View style={{ width: '100%', gap: t.space.sm, marginTop: -t.space.xxl }}>
        <Bubble tone="tutor" align="left" text="Hi! I'm Sana. Tell me about your day 🙂" />
        <Bubble tone="learner" align="right" text="I went to university and…" />
      </View>
    </View>
  );
}

function ArtBand() {
  const t = useTheme();
  const rows = [
    ['Fluency', 0.72],
    ['Vocabulary', 0.66],
    ['Grammar', 0.6],
  ] as const;
  return (
    <View style={{ alignItems: 'center', gap: t.space.xl, width: '100%' }}>
      <BandBadge band={6.5} size={124} />
      <View style={{ width: '100%', gap: t.space.md, backgroundColor: t.colors.surface, padding: t.space.lg, borderRadius: t.radius.lg, boxShadow: t.shadows.raised }}>
        {rows.map(([label, v]) => (
          <View key={label} style={{ gap: t.space.xs }}>
            <Txt variant="small" style={{ fontFamily: t.fonts.semibold }}>
              {label}
            </Txt>
            <ProgressBar value={v} />
          </View>
        ))}
      </View>
    </View>
  );
}

function ArtUrdu() {
  const t = useTheme();
  return (
    <View style={{ width: '100%', gap: t.space.md, backgroundColor: t.colors.surface, padding: t.space.xl, borderRadius: t.radius.xl, boxShadow: t.shadows.raised }}>
      <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.sm }}>
        <Icon name="close-circle" color={t.colors.danger} />
        <Txt style={{ textDecorationLine: 'line-through', color: t.colors.danger }}>I go to office yesterday</Txt>
      </View>
      <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.sm }}>
        <Icon name="checkmark-circle" color={t.colors.success} />
        <Txt variant="bodyStrong" style={{ color: t.colors.success }}>
          I went to the office yesterday
        </Txt>
      </View>
      <View style={{ backgroundColor: t.colors.accentSoft, borderRadius: t.radius.md, padding: t.space.md, gap: t.space.xs }}>
        <Txt variant="label" tone="muted">
          Explained in Urdu
        </Txt>
        <Txt style={{ fontSize: 18, lineHeight: 30, writingDirection: 'rtl', textAlign: 'right' }}>
          “کل” ماضی ہے، اس لیے go کی جگہ went بولیں۔
        </Txt>
      </View>
    </View>
  );
}

const SLIDES = [
  {
    art: <ArtTalk />,
    title: 'Speak with Sana, any time',
    body: 'A patient AI speaking coach for daily talk, job interviews and IELTS-style tests. No judgement, no appointments.',
  },
  {
    art: <ArtBand />,
    title: 'See your band grow',
    body: 'After every session you get an estimated band for fluency, vocabulary and grammar, with the exact lines to improve.',
  },
  {
    art: <ArtUrdu />,
    title: 'Mistakes explained in your language',
    body: 'Tap any correction to hear why, in Urdu or Hindi. Learn the rule once, use it forever.',
  },
];

export default function Onboarding() {
  const t = useTheme();
  const { width } = useWindowDimensions();
  const { finishOnboarding } = useOnboarding();
  const scroller = useRef<ComponentRef<typeof ScrollView>>(null);
  const [index, setIndex] = useState(0);
  const last = index === SLIDES.length - 1;

  const go = (i: number) => {
    scroller.current?.scrollTo({ x: i * width, animated: true });
    setIndex(i);
  };

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: t.colors.bg }}>
      <View pointerEvents="none" style={[StyleSheet.absoluteFill, { height: '55%', backgroundImage: t.gradients.wash }]} />
      <View style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', paddingHorizontal: t.space.lg, paddingTop: t.space.sm }}>
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.sm }}>
          <Gradient name="brand" style={{ width: 30, height: 30, borderRadius: 15, alignItems: 'center', justifyContent: 'center' }}>
            <Icon name="mic" size={16} color={t.colors.onBrand} />
          </Gradient>
          <Txt variant="heading">Voclara</Txt>
        </View>
        {!last && <Button title="Skip" variant="ghost" onPress={finishOnboarding} />}
      </View>

      <ScrollView
        ref={scroller}
        horizontal
        pagingEnabled
        showsHorizontalScrollIndicator={false}
        onMomentumScrollEnd={(e) => setIndex(Math.round(e.nativeEvent.contentOffset.x / width))}
        style={{ flex: 1 }}
      >
        {SLIDES.map((s, i) => (
          <View key={i} style={{ width, paddingHorizontal: t.space.xxl, justifyContent: 'center', gap: t.space.xxxl }}>
            <View style={{ alignItems: 'center', minHeight: 320, justifyContent: 'center' }}>{s.art}</View>
            <View style={{ gap: t.space.sm }}>
              <Txt variant="title" align="center" accessibilityRole="header">
                {s.title}
              </Txt>
              <Txt tone="muted" align="center">
                {s.body}
              </Txt>
            </View>
          </View>
        ))}
      </ScrollView>

      <View style={{ padding: t.space.xxl, gap: t.space.xl }}>
        <View style={{ flexDirection: 'row', justifyContent: 'center', gap: t.space.sm }} accessibilityLabel={`Page ${index + 1} of ${SLIDES.length}`}>
          {SLIDES.map((_, i) => (
            <View
              key={i}
              style={{
                width: i === index ? 26 : 8,
                height: 8,
                borderRadius: 4,
                backgroundColor: i === index ? t.colors.primary : t.colors.border,
              }}
            />
          ))}
        </View>
        <Button
          title={last ? 'Get started' : 'Next'}
          variant={last ? 'cta' : 'primary'}
          size="lg"
          icon={last ? 'sparkles' : 'arrow-forward'}
          onPress={() => (last ? finishOnboarding() : go(index + 1))}
        />
      </View>
    </SafeAreaView>
  );
}
