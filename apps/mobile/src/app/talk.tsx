/**
 * The live session: an immersive dark canvas with the tutor's orb, captions
 * and one big talk button. Logic (start, retry, complete) is unchanged from
 * before the redesign; see lib/live/session.ts for the protocol.
 */
import { router, useLocalSearchParams } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
import { useEffect, useRef, useState, type ComponentRef } from 'react';
import { Animated, AppState, Linking, Pressable, ScrollView, View } from 'react-native';
import { useAnimatedValue } from '@/components/anim';
import { SafeAreaView } from 'react-native-safe-area-context';

import { Button, Gradient, Icon, IconButton, SoundWave, Txt, VoiceOrb, type OrbState } from '@/components';
import { activeSessionId, forgetActiveSession, rememberActiveSession } from '@/lib/activeSession';
import { api, errorCode, errorMessage, type StartedSession } from '@/lib/api/client';
import { requestMicPermission } from '@/lib/audio/mic';
import { formatClock } from '@/lib/format';
import { setLastSession } from '@/lib/lastSession';
import {
  createLiveSession,
  type LiveResult,
  type LiveSession,
  type LiveState,
  type TalkMode,
} from '@/lib/live/session';
import type { Turn } from '@/lib/live/transcript';
import { useTheme } from '@/theme';

type Mode = 'talk' | 'mock' | 'interview';
type Phase =
  | { kind: 'preparing' }
  | { kind: 'denied' }
  | { kind: 'error'; message: string; outOfMinutes?: boolean }
  | { kind: 'running' }
  | { kind: 'saving' };

const MODE_LABEL: Record<Mode, string> = { talk: 'Talk with Sana', mock: 'IELTS-style mock', interview: 'Interview' };

function startSession(mode: Mode, topic?: string) {
  return api.POST('/api/v1/sessions', { body: { mode, topic: topic ?? null } });
}

export default function Talk() {
  const t = useTheme();
  const params = useLocalSearchParams<{ mode?: string; topic?: string; talkMode?: string }>();
  const mode: Mode = params.mode === 'mock' || params.mode === 'interview' ? params.mode : 'talk';
  const topic = params.topic || undefined;
  const initialTalkMode: TalkMode = params.talkMode === 'hands-free' ? 'hands-free' : 'push';

  const [phase, setPhase] = useState<Phase>({ kind: 'preparing' });
  const [liveState, setLiveState] = useState<LiveState>('connecting');
  const [turns, setTurns] = useState<Turn[]>([]);
  const [tutorSpeaking, setTutorSpeaking] = useState(false);
  const [level, setLevel] = useState(0);
  const [talkMode, setTalkMode] = useState<TalkMode>(initialTalkMode);
  const [holding, setHolding] = useState(false);
  const [secsLeft, setSecsLeft] = useState<number | null>(null);
  const [warning, setWarning] = useState(false);

  const sessionRef = useRef<LiveSession | null>(null);
  const startedRef = useRef<StartedSession | null>(null);
  const finishedRef = useRef(false);
  const captions = useRef<ComponentRef<typeof ScrollView>>(null);
  const ripple = useAnimatedValue(0);

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
        // OpenAI WebRTC and relay transports are not wired into the app yet.
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
        { connectUrl: started.connect_url, token: started.token, model: started.model, endsAt, mode: talkMode },
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

  // Ripple around the talk button while held.
  useEffect(() => {
    if (!holding) {
      ripple.setValue(0);
      return;
    }
    const loop = Animated.loop(Animated.timing(ripple, { toValue: 1, duration: 1100, useNativeDriver: true }));
    loop.start();
    return () => loop.stop();
  }, [holding, ripple]);

  useEffect(() => {
    captions.current?.scrollToEnd({ animated: true });
  }, [turns]);

  function changeTalkMode(m: TalkMode) {
    setTalkMode(m);
    sessionRef.current?.setMode(m);
  }

  // ---------------------------------------------------------------- blocked states

  if (phase.kind === 'denied' || phase.kind === 'error') {
    const denied = phase.kind === 'denied';
    return (
      <Gradient name="canvas" style={{ flex: 1 }}>
        <StatusBar style="light" />
        <SafeAreaView style={{ flex: 1, padding: t.space.xxl, justifyContent: 'center', gap: t.space.lg }}>
          <View style={{ alignItems: 'center', gap: t.space.md }}>
            <View
              style={{ width: 72, height: 72, borderRadius: 36, backgroundColor: t.colors.glass, alignItems: 'center', justifyContent: 'center' }}
            >
              <Icon name={denied ? 'mic-off' : phase.outOfMinutes ? 'hourglass' : 'cloud-offline'} size={34} color={t.colors.accent} />
            </View>
            <Txt variant="title" tone="onCanvas" align="center">
              {denied ? 'Microphone needed' : phase.outOfMinutes ? 'Out of minutes' : "Couldn't start"}
            </Txt>
            <Txt tone="onCanvasMuted" align="center">
              {denied
                ? 'Voclara needs the microphone to hear you speak. You can allow it in Settings.'
                : phase.outOfMinutes
                  ? 'You have used your practice minutes. Plans and top-ups are coming soon.'
                  : phase.message}
            </Txt>
          </View>
          {denied && <Button title="Open settings" variant="cta" icon="settings" onPress={() => Linking.openSettings()} />}
          <Button title="Go back" variant="secondary" onPress={() => router.back()} />
        </SafeAreaView>
      </Gradient>
    );
  }

  // ---------------------------------------------------------------- live

  const live = phase.kind === 'running' && liveState === 'live';
  const orbState: OrbState = tutorSpeaking ? 'speaking' : live && (talkMode === 'hands-free' || holding) ? 'listening' : 'idle';
  const status =
    phase.kind === 'preparing'
      ? 'Getting ready…'
      : phase.kind === 'saving'
        ? 'Saving your session…'
        : liveState === 'connecting'
          ? 'Connecting to Sana…'
          : liveState === 'reconnecting'
            ? 'Reconnecting…'
            : tutorSpeaking
              ? 'Sana is speaking'
              : talkMode === 'push'
                ? holding
                  ? 'Listening…'
                  : 'Hold the button and speak'
                : 'Listening… just talk';

  return (
    <Gradient name="canvas" style={{ flex: 1 }}>
      <StatusBar style="light" />
      <SafeAreaView style={{ flex: 1 }}>
        {/* Top bar */}
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.md, paddingHorizontal: t.space.lg, paddingTop: t.space.sm }}>
          <IconButton
            icon="close"
            label="End session"
            tone="canvas"
            onPress={() => (phase.kind === 'running' ? void sessionRef.current?.stop() : router.back())}
          />
          <View style={{ flex: 1, alignItems: 'center' }}>
            <Txt variant="label" tone="onCanvasMuted">
              {MODE_LABEL[mode]}
            </Txt>
            {topic && mode === 'talk' && (
              <Txt variant="small" tone="onCanvas" numberOfLines={1}>
                {topic}
              </Txt>
            )}
          </View>
          <View
            style={{
              minWidth: t.touch.min,
              paddingHorizontal: t.space.md,
              height: 36,
              borderRadius: t.radius.pill,
              alignItems: 'center',
              justifyContent: 'center',
              backgroundColor: warning ? t.colors.accent : t.colors.glass,
            }}
            accessibilityLabel={`Time left ${secsLeft === null ? 'unknown' : formatClock(secsLeft)}`}
          >
            <Txt variant="bodyStrong" style={{ color: warning ? t.colors.onAccent : t.colors.onCanvas, fontVariant: ['tabular-nums'] }}>
              {secsLeft === null ? '--:--' : formatClock(secsLeft)}
            </Txt>
          </View>
        </View>

        {/* Orb */}
        <View style={{ alignItems: 'center', marginTop: t.space.lg }}>
          <VoiceOrb size={150} state={orbState} level={tutorSpeaking ? 0 : level} />
          <View style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.sm, marginTop: -t.space.lg }}>
            {(orbState !== 'idle' || phase.kind !== 'running') && (
              <SoundWave bars={4} height={14} active color={tutorSpeaking ? t.colors.accent : t.colors.onCanvas} />
            )}
            <Txt tone="onCanvas" accessibilityLiveRegion="polite">
              {status}
            </Txt>
          </View>
        </View>

        {/* Captions */}
        <ScrollView
          ref={captions}
          style={{ flex: 1, marginTop: t.space.lg }}
          contentContainerStyle={{ paddingHorizontal: t.space.lg, gap: t.space.sm, paddingBottom: t.space.md }}
        >
          {turns.length === 0 && live && (
            <Txt variant="small" tone="onCanvasMuted" align="center">
              Captions appear here as you talk.
            </Txt>
          )}
          {turns.slice(-8).map((turn, i) => {
            const learner = turn.role === 'learner';
            return (
              <View
                key={`${turn.t_ms}-${i}`}
                style={{
                  alignSelf: learner ? 'flex-end' : 'flex-start',
                  maxWidth: '85%',
                  borderRadius: t.radius.lg,
                  borderBottomRightRadius: learner ? 4 : t.radius.lg,
                  borderBottomLeftRadius: learner ? t.radius.lg : 4,
                  paddingHorizontal: t.space.lg,
                  paddingVertical: t.space.md,
                  backgroundColor: learner ? t.colors.learnerBubble : t.colors.glass,
                }}
              >
                <Txt variant="caption" tone="onCanvasMuted">
                  {learner ? 'You' : 'Sana'}
                </Txt>
                <Txt tone="onCanvas">{turn.text}</Txt>
              </View>
            );
          })}
        </ScrollView>

        {/* Controls */}
        <View style={{ padding: t.space.lg, gap: t.space.lg, alignItems: 'center' }}>
          <View style={{ flexDirection: 'row', padding: t.space.xs, borderRadius: t.radius.pill, backgroundColor: t.colors.glass }}>
            {(['push', 'hands-free'] as const).map((m) => {
              const on = talkMode === m;
              return (
                <Pressable
                  key={m}
                  accessibilityRole="button"
                  accessibilityState={{ selected: on }}
                  onPress={() => changeTalkMode(m)}
                  style={{
                    paddingHorizontal: t.space.lg,
                    minHeight: 40,
                    justifyContent: 'center',
                    borderRadius: t.radius.pill,
                    backgroundColor: on ? t.colors.glassStrong : 'transparent',
                  }}
                >
                  <Txt variant="small" tone={on ? 'onCanvas' : 'onCanvasMuted'} style={{ fontFamily: t.fonts.semibold }}>
                    {m === 'push' ? 'Hold to talk' : 'Hands-free'}
                  </Txt>
                </Pressable>
              );
            })}
          </View>

          {talkMode === 'push' ? (
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
              style={{ alignItems: 'center', justifyContent: 'center', opacity: live ? 1 : 0.45 }}
            >
              <Animated.View
                style={{
                  position: 'absolute',
                  width: 104,
                  height: 104,
                  borderRadius: 52,
                  borderWidth: 3,
                  borderColor: t.colors.accent,
                  opacity: ripple.interpolate({ inputRange: [0, 1], outputRange: [0.7, 0] }),
                  transform: [{ scale: ripple.interpolate({ inputRange: [0, 1], outputRange: [1, 1.6] }) }],
                }}
              />
              <View
                style={{
                  width: 104,
                  height: 104,
                  borderRadius: 52,
                  alignItems: 'center',
                  justifyContent: 'center',
                  backgroundImage: holding ? t.gradients.sunrise : t.gradients.talk,
                  boxShadow: holding ? t.shadows.glow : t.shadows.raised,
                }}
              >
                <Icon name={holding ? 'mic' : 'mic-outline'} size={44} color={t.colors.onBrand} />
              </View>
            </Pressable>
          ) : (
            <Button
              title="End session"
              variant="secondary"
              icon="stop-circle"
              loading={phase.kind === 'saving'}
              disabled={phase.kind !== 'running'}
              onPress={() => void sessionRef.current?.stop()}
              style={{ alignSelf: 'stretch' }}
            />
          )}
          {talkMode === 'push' && (
            <Txt variant="caption" tone="onCanvasMuted">
              Tap ✕ to finish and get your feedback
            </Txt>
          )}
        </View>
      </SafeAreaView>
    </Gradient>
  );
}

