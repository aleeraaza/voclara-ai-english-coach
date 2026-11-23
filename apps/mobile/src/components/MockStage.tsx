/**
 * Mock-test overlay for the live screen (dark canvas): which part the
 * examiner is on and, in Part 2, the cue card with its prepare/speak clock.
 * Progress comes from the examiner's captions (lib/mock.ts).
 */
import { useEffect, useState } from 'react';
import { View } from 'react-native';

import type { CueCard } from '@/lib/api/client';
import { formatClock } from '@/lib/format';
import { LONG_TURN_SECS, PREP_SECS, longTurnStage, type MockPart, type MockProgress } from '@/lib/mock';
import { useTheme } from '@/theme';

import { Icon, Txt } from './core';

const PARTS: { part: MockPart; label: string }[] = [
  { part: 1, label: 'Part 1' },
  { part: 2, label: 'Part 2' },
  { part: 3, label: 'Part 3' },
];

export function MockStage({ card, progress }: { card: CueCard | null; progress: MockProgress }) {
  const t = useTheme();
  const [now, setNow] = useState(() => Date.now());
  const [prepStartedAt, setPrepStartedAt] = useState<number | null>(null);

  // Tick while the Part 2 clock matters; start it when the examiner says so.
  const clockOn = progress.part === 2 && progress.prepAnnounced;
  useEffect(() => {
    if (!clockOn) return;
    const id = setInterval(() => {
      const at = Date.now();
      setNow(at);
      setPrepStartedAt((p) => p ?? at);
    }, 500);
    return () => clearInterval(id);
  }, [clockOn]);

  const stage = longTurnStage(progress.part === 2 ? prepStartedAt : null, now);

  return (
    <View style={{ paddingHorizontal: t.space.lg, gap: t.space.md }}>
      {/* Stepper */}
      <View style={{ flexDirection: 'row', gap: t.space.sm }} accessibilityRole="progressbar" accessibilityLabel={`Part ${progress.part} of 3`}>
        {PARTS.map(({ part, label }) => {
          const done = part < progress.part || progress.ended;
          const current = part === progress.part && !progress.ended;
          return (
            <View
              key={part}
              style={{
                flex: 1,
                flexDirection: 'row',
                alignItems: 'center',
                justifyContent: 'center',
                gap: t.space.xs,
                minHeight: 32,
                borderRadius: t.radius.pill,
                backgroundColor: current ? t.colors.glassStrong : t.colors.glass,
                borderWidth: current ? 1 : 0,
                borderColor: t.colors.accent,
              }}
            >
              {done ? (
                <Icon name="checkmark-circle" size={16} color={t.colors.accent} />
              ) : current ? (
                <View style={{ width: 8, height: 8, borderRadius: 4, backgroundColor: t.colors.accent }} />
              ) : null}
              <Txt variant="caption" tone={current || done ? 'onCanvas' : 'onCanvasMuted'} style={{ fontFamily: t.fonts.semibold }}>
                {label}
              </Txt>
            </View>
          );
        })}
      </View>

      {/* Part 2: the cue card */}
      {progress.part === 2 && card && (
        <View style={{ backgroundColor: t.colors.glass, borderRadius: t.radius.lg, padding: t.space.lg, gap: t.space.sm }}>
          <View style={{ flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', gap: t.space.sm }}>
            <Txt variant="label" tone="onCanvasMuted">
              Cue card · {card.topic}
            </Txt>
            <LongTurnClock stage={stage} />
          </View>
          <Txt variant="bodyStrong" tone="onCanvas">
            {card.prompt}
          </Txt>
          <Txt variant="small" tone="onCanvasMuted">
            You should say:
          </Txt>
          {card.bullets.map((b) => (
            <View key={b} style={{ flexDirection: 'row', gap: t.space.sm }}>
              <Txt variant="small" tone="onCanvasMuted">
                •
              </Txt>
              <Txt variant="small" tone="onCanvas" style={{ flex: 1 }}>
                {b}
              </Txt>
            </View>
          ))}
          <Txt variant="caption" tone="onCanvasMuted">
            {stage.kind === 'prepare'
              ? 'Plan your answer. Think of one example for each point.'
              : stage.kind === 'speak'
                ? 'Speak now. Keep going until the examiner stops you.'
                : stage.kind === 'done'
                  ? 'Time is up. Finish your sentence.'
                  : 'Listen to the examiner.'}
          </Txt>
        </View>
      )}

      {/* Part 3: a reminder of the Part 2 topic */}
      {progress.part === 3 && card && !progress.ended && (
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.sm }}>
          <Icon name="chatbubbles" size={16} color={t.colors.onCanvasMuted} />
          <Txt variant="small" tone="onCanvasMuted" numberOfLines={1} style={{ flex: 1 }}>
            Discussion linked to: {card.prompt}
          </Txt>
        </View>
      )}
    </View>
  );
}

function LongTurnClock({ stage }: { stage: ReturnType<typeof longTurnStage> }) {
  const t = useTheme();
  if (stage.kind === 'waiting') return null;
  const prepare = stage.kind === 'prepare';
  const done = stage.kind === 'done';
  const secs = stage.kind === 'prepare' || stage.kind === 'speak' ? stage.secsLeft : 0;
  const total = prepare ? PREP_SECS : LONG_TURN_SECS;
  return (
    <View
      accessibilityLabel={done ? 'Time is up' : `${prepare ? 'Prepare' : 'Speak'}, ${secs} seconds left`}
      accessibilityLiveRegion="polite"
      style={{
        flexDirection: 'row',
        alignItems: 'center',
        gap: t.space.xs,
        paddingHorizontal: t.space.md,
        minHeight: 32,
        borderRadius: t.radius.pill,
        backgroundColor: prepare ? t.colors.accent : t.colors.glassStrong,
      }}
    >
      <Icon name={prepare ? 'hourglass' : 'mic'} size={14} color={prepare ? t.colors.onAccent : t.colors.onCanvas} />
      <Txt
        variant="caption"
        style={{
          color: prepare ? t.colors.onAccent : t.colors.onCanvas,
          fontFamily: t.fonts.bold,
          fontVariant: ['tabular-nums'],
        }}
      >
        {done ? 'Time' : `${prepare ? 'Prepare' : 'Speak'} ${formatClock(secs)}`}
      </Txt>
      {!done && (
        <Txt variant="caption" style={{ color: prepare ? t.colors.onAccentMuted : t.colors.onCanvasMuted }}>
          / {formatClock(total)}
        </Txt>
      )}
    </View>
  );
}
