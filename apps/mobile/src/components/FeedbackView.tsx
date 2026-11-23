/**
 * Graded feedback for one session: band medallion, criteria, corrections
 * (with "Explain in <language>"), speaking stats and next focus. Polls while
 * grading is pending. Used right after a session and from history.
 */
import { useEffect, useState } from 'react';
import { ActivityIndicator, Pressable, View } from 'react-native';

import { api, type Feedback } from '@/lib/api/client';
import { languageLabel } from '@/lib/labels';
import { shareResult } from '@/lib/shareCard';
import { pollUntil } from '@/lib/poll';
import { useTheme } from '@/theme';

import { Button, Card, Icon, Txt } from './core';
import { BandBadge, EmptyState, ProgressBar, SectionTitle } from './widgets';

const CRITERIA = [
  { key: 'fluency', label: 'Fluency & coherence', icon: 'pulse' },
  { key: 'lexical', label: 'Vocabulary', icon: 'book' },
  { key: 'grammar', label: 'Grammar', icon: 'construct' },
  { key: 'pronunciation', label: 'Pronunciation', icon: 'volume-high' },
] as const;

function headline(band: number): string {
  if (band >= 7) return 'Excellent work!';
  if (band >= 6) return 'Good progress!';
  if (band >= 5) return 'Nice effort, keep going!';
  return 'Every session makes you stronger.';
}

/** The API answered, but not with feedback (e.g. 404): polling again won't help. */
class NoFeedback extends Error {}

export function useFeedback(sessionId: string | null, enabled = true) {
  const [fb, setFb] = useState<Feedback | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    if (!sessionId || !enabled) return;
    let cancelled = false;
    pollUntil(
      async () => {
        const { data, response } = await api.GET('/api/v1/sessions/{id}/feedback', {
          params: { path: { id: sessionId } },
        });
        if (!data) throw new NoFeedback(`feedback request failed (${response.status})`);
        return data;
      },
      {
        done: (v) => v.status !== 'pending',
        onValue: (v) => {
          if (!cancelled) setFb(v);
        },
        cancelled: () => cancelled,
      },
    )
      .then((last) => {
        // pollUntil also returns when it times out; a still-pending result
        // means grading is stuck, so stop the spinner and offer a retry.
        if (!cancelled && last.status === 'pending') {
          setError('Your review is taking longer than usual. Try again in a minute.');
        }
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setError(
          e instanceof NoFeedback
            ? 'Feedback is not available for this session.'
            : 'Could not load feedback. Check your connection.',
        );
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, sessionId, attempt]);
  return {
    fb,
    error,
    retry: () => {
      setError(null);
      setAttempt((n) => n + 1);
    },
  };
}

export function FeedbackView({
  fb,
  error,
  onRetry,
}: {
  fb: Feedback | null;
  error: string | null;
  onRetry?: () => void;
}) {
  const t = useTheme();
  const [openWhy, setOpenWhy] = useState<Record<number, boolean>>({});

  if (error) {
    return (
      <Card>
        <EmptyState icon="cloud-offline" title="Feedback unavailable" body={error} />
        {onRetry ? <Button title="Try again" variant="secondary" icon="refresh" onPress={onRetry} /> : null}
      </Card>
    );
  }

  if (!fb || fb.status === 'pending') {
    return (
      <Card style={{ alignItems: 'center', paddingVertical: t.space.xxl }}>
        <ActivityIndicator color={t.colors.primary} size="large" />
        <Txt variant="heading" align="center">
          Sana is reviewing your answers…
        </Txt>
        <Txt tone="muted" align="center">
          Usually under a minute. You can stay here or come back from Progress.
        </Txt>
      </Card>
    );
  }

  if (fb.status === 'skipped') {
    return (
      <Card>
        <EmptyState
          icon="chatbubble-ellipses"
          title="A little more speaking next time"
          body={`We need at least ${fb.min_words} words from you to estimate a band. Try longer answers with a reason or an example.`}
        />
      </Card>
    );
  }

  if (fb.status !== 'ready' || !fb.bands) {
    return (
      <Card>
        <EmptyState icon="cloud-offline" title="Feedback unavailable" body="We could not grade this session. Your practice still counts." />
      </Card>
    );
  }

  const l1 = fb.l1 && fb.l1 !== 'en' ? languageLabel(fb.l1) : null;
  const bands = fb.bands;
  const isMock = fb.mode === 'mock';
  // A mock result keeps it focused: the three fixes that matter most.
  const corrections = isMock ? fb.corrections.slice(0, 3) : fb.corrections;
  const share = () => void shareResult(fb.session_id, bands.overall, isMock).catch(() => undefined);

  return (
    <>
      {/* Hero */}
      <Card style={{ alignItems: 'center', gap: t.space.md, paddingVertical: t.space.xxl }}>
        {fb.demo && (
          <View style={{ backgroundColor: t.colors.surfaceAlt, borderRadius: t.radius.pill, paddingHorizontal: t.space.md, paddingVertical: 2 }}>
            <Txt variant="label" tone="muted">
              Demo feedback
            </Txt>
          </View>
        )}
        <BandBadge band={bands.overall} size={132} label="" />
        <Txt variant="title" align="center">
          {headline(bands.overall)}
        </Txt>
        <Txt variant="small" tone="muted" align="center">
          {isMock ? 'Estimated overall band · IELTS-style mock' : 'Estimated overall band · IELTS-style practice'}
        </Txt>
        {!fb.demo && <Button title="Share result" variant="secondary" icon="share-social" onPress={share} />}
      </Card>

      {fb.cue_card ? (
        <Card tone="soft" style={{ gap: t.space.xs }}>
          <Txt variant="label" tone="primary">
            {`Your Part 2 task · ${fb.cue_card.topic}`}
          </Txt>
          <Txt variant="bodyStrong">{fb.cue_card.prompt}</Txt>
        </Card>
      ) : null}

      {/* Criteria */}
      <SectionTitle>Your four criteria</SectionTitle>
      <Card style={{ gap: t.space.lg }}>
        {CRITERIA.map((c) => {
          const value = bands[c.key];
          const quote = fb.evidence?.[c.key]?.[0];
          return (
            <View key={c.key} style={{ gap: t.space.xs }}>
              <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.sm }}>
                <Icon name={c.icon} size={18} color={t.colors.primary} />
                <Txt variant="bodyStrong" style={{ flex: 1 }}>
                  {c.label}
                </Txt>
                <Txt variant="bodyStrong" tone={value === null ? 'muted' : 'primary'}>
                  {value === null || value === undefined ? '–' : value.toFixed(1)}
                </Txt>
              </View>
              {value === null || value === undefined ? (
                <Txt variant="caption" tone="muted">
                  Pronunciation scoring is coming soon.
                </Txt>
              ) : (
                <>
                  <ProgressBar value={value / 9} />
                  {quote ? (
                    <Txt variant="caption" tone="muted">
                      “{quote}”
                    </Txt>
                  ) : null}
                </>
              )}
            </View>
          );
        })}
      </Card>

      {/* Corrections */}
      {corrections.length > 0 && (
        <>
          <SectionTitle>
            {isMock
              ? `Top ${corrections.length} ${corrections.length === 1 ? 'fix' : 'fixes'}`
              : `${corrections.length} ${corrections.length === 1 ? 'fix' : 'fixes'} to try`}
          </SectionTitle>
          {corrections.map((c, i) => (
            <Card key={i} style={{ gap: t.space.sm }}>
              <View style={{ flexDirection: 'row', gap: t.space.sm }}>
                <Icon name="close-circle" size={20} color={t.colors.danger} />
                <Txt style={{ flex: 1, color: t.colors.danger, textDecorationLine: 'line-through' }}>{c.said}</Txt>
              </View>
              <View style={{ flexDirection: 'row', gap: t.space.sm }}>
                <Icon name="checkmark-circle" size={20} color={t.colors.success} />
                <Txt variant="bodyStrong" style={{ flex: 1, color: t.colors.success }}>
                  {c.better}
                </Txt>
              </View>
              <Txt variant="small" tone="muted">
                {c.why_en}
              </Txt>
              {l1 && c.why_l1 ? (
                <>
                  <Pressable
                    accessibilityRole="button"
                    accessibilityState={{ expanded: !!openWhy[i] }}
                    onPress={() => setOpenWhy((o) => ({ ...o, [i]: !o[i] }))}
                    style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.xs, minHeight: 40 }}
                  >
                    <Icon name="language" size={18} color={t.colors.primary} />
                    <Txt variant="small" tone="primary" style={{ fontFamily: t.fonts.bold }}>
                      {openWhy[i] ? 'Hide explanation' : `Explain in ${l1}`}
                    </Txt>
                  </Pressable>
                  {openWhy[i] && (
                    <View style={{ backgroundColor: t.colors.accentSoft, borderRadius: t.radius.md, padding: t.space.md }}>
                      <Txt style={{ fontFamily: undefined, fontSize: 18, lineHeight: 30, writingDirection: 'auto' }}>{c.why_l1}</Txt>
                    </View>
                  )}
                </>
              ) : null}
            </Card>
          ))}
        </>
      )}

      {/* Stats */}
      <SectionTitle>How you spoke</SectionTitle>
      <View style={{ flexDirection: 'row', gap: t.space.md }}>
        <Card style={{ flex: 1, gap: t.space.xs }}>
          <Icon name="speedometer" color={t.colors.info} />
          <Txt variant="title">{fb.wpm ?? '–'}</Txt>
          <Txt variant="caption" tone="muted">
            Words per minute
          </Txt>
        </Card>
        <Card style={{ flex: 1, gap: t.space.xs }}>
          <Icon name="chatbubble-ellipses" color={t.colors.accent} />
          <Txt variant="title">{fb.fillers?.total ?? 0}</Txt>
          <Txt variant="caption" tone="muted" numberOfLines={2}>
            {fb.fillers && fb.fillers.top.length > 0
              ? `Fillers: ${fb.fillers.top.map((f) => `“${f.word}”`).join(', ')}`
              : 'Filler words'}
          </Txt>
        </Card>
      </View>

      {fb.next_focus ? (
        <Card tone="soft" style={{ flexDirection: 'row', gap: t.space.md, alignItems: 'center' }}>
          <Icon name="flag" color={t.colors.primary} />
          <View style={{ flex: 1 }}>
            <Txt variant="label" tone="primary">
              Practise next
            </Txt>
            <Txt variant="bodyStrong">{fb.next_focus}</Txt>
          </View>
        </Card>
      ) : null}
    </>
  );
}
