import { router, useLocalSearchParams } from 'expo-router';
import { useEffect, useRef, useState } from 'react';
import { AppState, Linking, Pressable, StyleSheet, Text, View } from 'react-native';

import { Body, Button, Card, Chips, ErrorText, Screen, formatClock, useTheme } from '@/components/ui';
import { activeSessionId, forgetActiveSession, rememberActiveSession } from '@/lib/activeSession';
import { api, errorCode, errorMessage, type StartedSession } from '@/lib/api/client';
import { requestMicPermission } from '@/lib/audio/mic';
import { setLastSession } from '@/lib/lastSession';
import {
  createLiveSession,
  type LiveResult,
  type LiveSession,
  type LiveState,
  type TalkMode,
} from '@/lib/live/session';
import type { Turn } from '@/lib/live/transcript';

type Mode = 'talk' | 'mock' | 'interview';
type Phase =
  | { kind: 'preparing' }
  | { kind: 'denied' }
  | { kind: 'error'; message: string; outOfMinutes?: boolean }
  | { kind: 'running' }
  | { kind: 'saving' };

const MODE_LABEL: Record<Mode, string> = { talk: 'Talk with Sana', mock: 'IELTS-style mock', interview: 'Interview' };
const TALK_MODES = [
  { value: 'push', label: 'Hold to talk' },
  { value: 'hands-free', label: 'Hands-free' },
] as const;

function startSession(mode: Mode, topic?: string) {
  return api.POST('/api/v1/sessions', { body: { mode, topic: topic ?? null } });
}

export default function Talk() {
  const t = useTheme();
  const params = useLocalSearchParams<{ mode?: string; topic?: string }>();
  const mode: Mode = params.mode === 'mock' || params.mode === 'interview' ? params.mode : 'talk';
  const topic = params.topic || undefined;

  const [phase, setPhase] = useState<Phase>({ kind: 'preparing' });
  const [liveState, setLiveState] = useState<LiveState>('connecting');
  const [turns, setTurns] = useState<Turn[]>([]);
  const [tutorSpeaking, setTutorSpeaking] = useState(false);
  const [level, setLevel] = useState(0);
  const [talkMode, setTalkMode] = useState<TalkMode>('push');
  const [holding, setHolding] = useState(false);
  const [secsLeft, setSecsLeft] = useState<number | null>(null);
  const [warning, setWarning] = useState(false);

  const sessionRef = useRef<LiveSession | null>(null);
  const startedRef = useRef<StartedSession | null>(null);
  const finishedRef = useRef(false);

  /** Upload the transcript, charge the time, then show the summary. */
  async function finalize(result: LiveResult) {
    if (finishedRef.current) return;
    finishedRef.current = true;
    const started = startedRef.current;
    if (!started) return;
    setPhase({ kind: 'saving' });
    const { data, error } = await api
      .POST('/api/v1/sessions/{id}/complete', {
        params: { path: { id: started.session_id } },
        body: { transcript: result.transcript },
      })
      .catch(() => ({ data: undefined, error: { error: { message: 'Could not reach Voclara to save the session.' } } }));
    if (data || errorCode(error) === 'conflict') await forgetActiveSession();
    setLastSession({
      sessionId: started.session_id,
      mode,
      reason: result.reason,
      secondsCharged: data?.seconds_charged ?? null,
      balanceSecs: data?.balance_secs ?? null,
      transcript: result.transcript,
      recordingPath: result.recordingPath,
      uploadError: error ? errorMessage(error) : undefined,
      liveError: result.error,
    });
    router.replace('/feedback');
  }

  // Start once on mount.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      if ((await requestMicPermission()) === 'denied') {
        if (!cancelled) setPhase({ kind: 'denied' });
        return;
      }

      let res = await startSession(mode, topic).catch(() => null);
      // A session left running by a crash blocks new ones: close it and retry.
      if (res?.response.status === 409) {
        const stale = await activeSessionId();
        if (stale) {
          await api
            .POST('/api/v1/sessions/{id}/complete', { params: { path: { id: stale } }, body: {} })
            .catch(() => null);
          await forgetActiveSession();
          res = await startSession(mode, topic).catch(() => null);
        }
      }
      if (cancelled) return;
      if (!res) return setPhase({ kind: 'error', message: 'Cannot reach Voclara. Check your internet.' });
      if (!res.data) {
        return setPhase({
          kind: 'error',
          message:
            res.response.status === 409
              ? 'Another session is still running. Try again in a few minutes.'
              : errorMessage(res.error),
          outOfMinutes: res.response.status === 402,
        });
      }

      const started = res.data;
      startedRef.current = started;
      await rememberActiveSession(started.session_id);

      if (started.transport !== 'websocket') {
        // OpenAI WebRTC fallback is not wired into the app yet.
        await finalize({
          reason: 'error',
          transcript: [],
          recordingPath: null,
          durationMs: 0,
          error: 'This voice provider is not supported by this version of the app.',
        });
        return;
      }

      const endsAt = Date.parse(started.session_ends_at);
      const session = createLiveSession(
        {
          connectUrl: started.connect_url,
          token: started.token,
          model: started.model,
          endsAt,
          mode: talkMode,
        },
        {
          onState: setLiveState,
          onTranscript: setTurns,
          onTutorSpeaking: setTutorSpeaking,
          onLevel: setLevel,
          onTimeWarning: () => setWarning(true),
          onEnd: (r) => void finalize(r),
        },
      );
      sessionRef.current = session;
      setPhase({ kind: 'running' });
      setSecsLeft(Math.round((endsAt - Date.now()) / 1000));
      session.start();
    })();

    return () => {
      cancelled = true;
      // Leaving the screen ends the session (and saves it).
      void sessionRef.current?.stop();
    };
    // Start exactly once; mode/topic come from the route and do not change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Countdown.
  useEffect(() => {
    if (phase.kind !== 'running' || !startedRef.current) return;
    const endsAt = Date.parse(startedRef.current.session_ends_at);
    const id = setInterval(() => setSecsLeft(Math.max(0, Math.round((endsAt - Date.now()) / 1000))), 1000);
    return () => clearInterval(id);
  }, [phase.kind]);

  // Going to the background ends the session: the mic cannot keep running there.
  useEffect(() => {
    const sub = AppState.addEventListener('change', (s) => {
      if (s === 'background') void sessionRef.current?.stop();
    });
    return () => sub.remove();
  }, []);

  function changeTalkMode(m: TalkMode) {
    setTalkMode(m);
    sessionRef.current?.setMode(m);
  }

  if (phase.kind === 'denied') {
    return (
      <Screen>
        <Card>
          <Body>Voclara needs the microphone to hear you speak.</Body>
          <Button title="Open settings" onPress={() => Linking.openSettings()} />
          <Button title="Back" variant="secondary" onPress={() => router.back()} />
        </Card>
      </Screen>
    );
  }

  if (phase.kind === 'error') {
    return (
      <Screen>
        <Card>
          <ErrorText>{phase.message}</ErrorText>
          {phase.outOfMinutes && <Body muted>Plans and minute top-ups are coming soon.</Body>}
          <Button title="Back" variant="secondary" onPress={() => router.back()} />
        </Card>
      </Screen>
    );
  }

  const status =
    phase.kind === 'preparing'
      ? 'Getting ready…'
      : phase.kind === 'saving'
        ? 'Saving your session…'
        : liveState === 'connecting'
          ? 'Connecting…'
          : liveState === 'reconnecting'
            ? 'Reconnecting…'
            : tutorSpeaking
              ? 'Sana is speaking'
              : talkMode === 'push'
                ? holding
                  ? 'Listening…'
                  : 'Hold the button and speak'
                : 'Listening…';

  const live = phase.kind === 'running' && liveState === 'live';

  return (
    <Screen scroll={false}>
      <View style={styles.header}>
        <Body muted>{MODE_LABEL[mode]}</Body>
        <Text style={[styles.clock, { color: warning ? t.danger : t.text }]}>
          {secsLeft === null ? '--:--' : formatClock(secsLeft)}
        </Text>
      </View>

      <Card style={{ flex: 1 }}>
        <Body muted>{status}</Body>
        <View style={{ flex: 1, justifyContent: 'flex-end', gap: 8 }}>
          {turns.slice(-6).map((turn, i) => (
            <Text
              key={`${turn.t_ms}-${i}`}
              style={[
                styles.caption,
                turn.role === 'learner'
                  ? { alignSelf: 'flex-end', backgroundColor: t.primary, color: t.primaryText }
                  : { alignSelf: 'flex-start', backgroundColor: t.bg, color: t.text },
              ]}
            >
              {turn.text}
            </Text>
          ))}
        </View>
        <View style={[styles.meterTrack, { backgroundColor: t.border }]}>
          <View
            style={[
              styles.meterFill,
              { backgroundColor: t.success, width: `${Math.min(100, Math.round(level * 400))}%` },
            ]}
          />
        </View>
      </Card>

      <Chips options={TALK_MODES} value={talkMode} onChange={changeTalkMode} />
      {talkMode === 'push' && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Hold to talk"
          disabled={!live}
          onPressIn={() => {
            setHolding(true);
            sessionRef.current?.setTalking(true);
          }}
          onPressOut={() => {
            setHolding(false);
            sessionRef.current?.setTalking(false);
          }}
          style={[
            styles.ptt,
            { backgroundColor: holding ? t.success : t.primary, opacity: live ? 1 : 0.5 },
          ]}
        >
          <Text style={[styles.pttText, { color: t.primaryText }]}>{holding ? 'Listening…' : 'Hold to talk'}</Text>
        </Pressable>
      )}
      <Button
        title="End session"
        variant="danger"
        loading={phase.kind === 'saving'}
        disabled={phase.kind !== 'running'}
        onPress={() => void sessionRef.current?.stop()}
      />
    </Screen>
  );
}

const styles = StyleSheet.create({
  header: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' },
  clock: { fontSize: 28, fontWeight: '700', fontVariant: ['tabular-nums'] },
  caption: { maxWidth: '85%', borderRadius: 14, paddingHorizontal: 12, paddingVertical: 8, fontSize: 16, overflow: 'hidden' },
  meterTrack: { height: 6, borderRadius: 3, overflow: 'hidden' },
  meterFill: { height: 6 },
  ptt: { height: 96, borderRadius: 48, alignItems: 'center', justifyContent: 'center' },
  pttText: { fontSize: 18, fontWeight: '700' },
});
