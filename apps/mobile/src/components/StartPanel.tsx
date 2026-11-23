/** Footer of every practice setup screen: talk mode, minutes left, Start. */
import { router } from 'expo-router';
import { View } from 'react-native';

import { useAuth } from '@/lib/auth';
import { formatMinutes } from '@/lib/format';
import { useTalkMode } from '@/lib/talkPrefs';
import { useTheme } from '@/theme';

import { Button, Card, Txt } from './core';
import { ChipGroup, Notice } from './widgets';

const TALK_MODES = [
  { value: 'push', label: 'Hold to talk', icon: 'hand-left' },
  { value: 'hands-free', label: 'Hands-free', icon: 'headset' },
] as const;

interface StartPanelProps {
  mode: 'talk' | 'mock' | 'interview';
  topic?: string;
  /** Mode limit in minutes, for the copy. */
  limitMins: number;
  disabled?: boolean;
}

export function StartPanel({ mode, topic, limitMins, disabled }: StartPanelProps) {
  const t = useTheme();
  const { me } = useAuth();
  const [talkMode, setTalkMode] = useTalkMode();
  const balance = me?.minutes.balance_secs ?? 0;
  const out = me !== null && balance < 30;
  const mins = Math.min(limitMins, Math.floor(balance / 60));

  return (
    <Card style={{ gap: t.space.lg }}>
      <View style={{ gap: t.space.sm }}>
        <Txt variant="label" tone="muted">
          How you’ll talk
        </Txt>
        <ChipGroup options={TALK_MODES} value={talkMode} onChange={setTalkMode} />
        <Txt variant="caption" tone="muted">
          {talkMode === 'push'
            ? 'Hold the button while you speak. Best in noisy places.'
            : 'Just talk, like a phone call. Best with earphones.'}
        </Txt>
      </View>
      {out ? (
        <Notice tone="info">You have used your free minutes. Plans and top-ups are coming soon.</Notice>
      ) : (
        <Txt variant="small" tone="muted">
          Up to {mins} min this session · {formatMinutes(balance)} left
        </Txt>
      )}
      <Button
        title="Start session"
        variant="cta"
        size="lg"
        icon="mic"
        disabled={disabled || out}
        onPress={() =>
          router.push({
            pathname: '/talk',
            params: topic ? { mode, topic, talkMode } : { mode, talkMode },
          })
        }
      />
    </Card>
  );
}
