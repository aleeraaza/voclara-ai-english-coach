import { router } from 'expo-router';
import { useEffect, useState } from 'react';
import { ActivityIndicator, Pressable, StyleSheet, Text, View } from 'react-native';

import { Body, Button, Card, ErrorText, Screen, Title, formatMinutes, useTheme } from '@/components/ui';
import { api, type Feedback as FeedbackData } from '@/lib/api/client';
import { getLastSession } from '@/lib/lastSession';
import { pollUntil } from '@/lib/poll';

const REASON: Record<string, string> = {
  user: 'You ended the session.',
  time_up: 'Time is up for this session.',
  connection_lost: 'The connection dropped, so the session ended.',
  error: 'The session could not continue.',
};

const LANGUAGE: Record<string, string> = {
  ur: 'Urdu',
  hi: 'Hindi',
  bn: 'Bengali',
  ar: 'Arabic',
};

const CRITERIA = [
  { key: 'fluency', label: 'Fluency & coherence' },
  { key: 'lexical', label: 'Vocabulary' },
  { key: 'grammar', label: 'Grammar' },
  { key: 'pronunciation', label: 'Pronunciation' },
] as const;

function band(v: number | null | undefined): string {
  return v === null || v === undefined ? '–' : v.toFixed(1);
}

export default function Feedback() {
  const t = useTheme();
  const s = getLastSession();
  const [fb, setFb] = useState<FeedbackData | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [openWhy, setOpenWhy] = useState<Record<number, boolean>>({});

  // Grading runs in the background after the session is saved: poll for it.
  useEffect(() => {
    if (!s || s.secondsCharged === null) return;
    let cancelled = false;
    pollUntil(
      async () => {
        const { data } = await api.GET('/api/v1/sessions/{id}/feedback', {
          params: { path: { id: s.sessionId } },
        });
        return data ?? null;
      },
      {
        done: (v) => v !== null && v.status !== 'pending',
        onValue: (v) => {
          if (!cancelled && v) setFb(v);
        },
        cancelled: () => cancelled,
      },
    ).catch(() => {
      if (!cancelled) setLoadError('Could not load feedback. Check your connection.');
    });
    return () => {
      cancelled = true;
    };
  }, [s]);

  if (!s) {
    return (
      <Screen>
        <Body muted>No session to show.</Body>
        <Button title="Home" onPress={() => router.replace('/')} />
      </Screen>
    );
  }

  const learnerTurns = s.transcript.filter((x) => x.role === 'learner');
  const words = learnerTurns.reduce((n, x) => n + x.text.split(/\s+/).filter(Boolean).length, 0);
  const l1Name = fb?.l1 ? LANGUAGE[fb.l1] : undefined;
  const pending = s.secondsCharged !== null && (!fb || fb.status === 'pending') && !loadError;

  return (
    <Screen>
      <Card>
        <Title>Session saved</Title>
        <Body muted>{REASON[s.reason] ?? ''}</Body>
        {s.secondsCharged !== null && <Body>Time used: {formatMinutes(s.secondsCharged)}</Body>}
        {s.balanceSecs !== null && <Body>Minutes left: {formatMinutes(s.balanceSecs)}</Body>}
        <Body>
          You spoke {learnerTurns.length} {learnerTurns.length === 1 ? 'time' : 'times'}, about {words} words.
        </Body>
        <ErrorText>{s.liveError}</ErrorText>
        <ErrorText>{s.uploadError}</ErrorText>
      </Card>

      {pending && (
        <Card>
          <View style={styles.row}>
            <ActivityIndicator color={t.primary} />
            <Body>Sana is reviewing your answers…</Body>
          </View>
        </Card>
      )}
      <ErrorText>{loadError}</ErrorText>

      {fb?.status === 'skipped' && (
        <Card>
          <Title>Not enough speech to grade</Title>
          <Body muted>
            We need at least {fb.min_words} words from you to estimate a band. Try a longer answer next time.
          </Body>
        </Card>
      )}
      {fb?.status === 'failed' && (
        <Card>
          <Title>Feedback unavailable</Title>
          <Body muted>We could not grade this session. Your practice still counts.</Body>
        </Card>
      )}

      {fb?.status === 'ready' && fb.bands && (
        <>
          <Card>
            {fb.demo && (
              <Text style={[styles.badge, { backgroundColor: t.border, color: t.text }]}>DEMO FEEDBACK</Text>
            )}
            <Body muted>Estimated band (IELTS-style)</Body>
            <Text style={[styles.overall, { color: t.primary }]}>{band(fb.bands.overall)}</Text>
            {CRITERIA.map((c) => {
              const value = fb.bands?.[c.key];
              const quote = fb.evidence?.[c.key]?.[0];
              return (
                <View key={c.key} style={styles.criterion}>
                  <View style={styles.row}>
                    <Body style={{ flex: 1 }}>{c.label}</Body>
                    <Body style={{ fontWeight: '700' }}>{band(value)}</Body>
                  </View>
                  {value === null && c.key === 'pronunciation' ? (
                    <Body muted style={styles.small}>Pronunciation scoring is coming soon.</Body>
                  ) : (
                    quote && <Body muted style={styles.small}>“{quote}”</Body>
                  )}
                </View>
              );
            })}
          </Card>

          {fb.corrections.length > 0 && (
            <Card>
              <Title>Corrections</Title>
              {fb.corrections.map((c, i) => (
                <View key={i} style={[styles.correction, { borderColor: t.border }]}>
                  <Text style={[styles.said, { color: t.danger }]}>{c.said}</Text>
                  <Text style={[styles.better, { color: t.success }]}>{c.better}</Text>
                  <Body muted style={styles.small}>{c.why_en}</Body>
                  {l1Name && c.why_l1 ? (
                    <Pressable
                      accessibilityRole="button"
                      onPress={() => setOpenWhy((o) => ({ ...o, [i]: !o[i] }))}
                    >
                      <Text style={[styles.link, { color: t.primary }]}>
                        {openWhy[i] ? 'Hide' : `Explain in ${l1Name}`}
                      </Text>
                    </Pressable>
                  ) : null}
                  {openWhy[i] && <Body style={styles.l1}>{c.why_l1}</Body>}
                </View>
              ))}
            </Card>
          )}

          <Card>
            <Title>How you spoke</Title>
            {fb.wpm !== null && fb.wpm !== undefined && <Body>Speed: about {fb.wpm} words per minute</Body>}
            {fb.fillers && (
              <Body>
                Filler words: {fb.fillers.total}
                {fb.fillers.top.length > 0
                  ? ` (${fb.fillers.top.map((f) => `“${f.word}” ×${f.count}`).join(', ')})`
                  : ''}
              </Body>
            )}
            {fb.next_focus && (
              <>
                <Body muted style={{ marginTop: 8 }}>Practise next</Body>
                <Body style={{ fontWeight: '700' }}>{fb.next_focus}</Body>
              </>
            )}
          </Card>
        </>
      )}

      {learnerTurns.length > 0 && (
        <Card>
          <Title>What you said</Title>
          {learnerTurns.map((x, i) => (
            <Body key={i}>• {x.text}</Body>
          ))}
        </Card>
      )}

      <Body muted style={styles.small}>
        Bands are estimates for IELTS-style practice. Voclara is not affiliated with IELTS.
      </Body>
      <Button title="Back to home" onPress={() => router.replace('/')} />
    </Screen>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: 10 },
  overall: { fontSize: 56, fontWeight: '800', fontVariant: ['tabular-nums'] },
  criterion: { gap: 2, paddingVertical: 4 },
  small: { fontSize: 13, lineHeight: 18 },
  badge: {
    alignSelf: 'flex-start',
    fontSize: 11,
    fontWeight: '800',
    letterSpacing: 1,
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 6,
    overflow: 'hidden',
  },
  correction: { borderTopWidth: StyleSheet.hairlineWidth, paddingTop: 10, gap: 4 },
  said: { fontSize: 15, textDecorationLine: 'line-through' },
  better: { fontSize: 16, fontWeight: '700' },
  link: { fontSize: 14, fontWeight: '700', paddingVertical: 4 },
  l1: { writingDirection: 'auto', fontSize: 17, lineHeight: 26 },
});
