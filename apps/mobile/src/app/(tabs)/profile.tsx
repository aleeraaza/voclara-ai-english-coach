import { router } from 'expo-router';
import { View } from 'react-native';

import { Card, Gradient, ListGroup, ListRow, ProgressBar, Screen, SectionTitle, Txt } from '@/components';
import { useAuth } from '@/lib/auth';
import { formatMinutes } from '@/lib/format';
import { goalLabel, languageLabel, levelLabel } from '@/lib/labels';
import { useTheme } from '@/theme';

/** The free plan's one-time grant, for the minutes bar. */
const FREE_SECS = 600;

export default function Profile() {
  const t = useTheme();
  const { me, signOut } = useAuth();
  const balance = me?.minutes.balance_secs ?? 0;
  const initial = (me?.name ?? 'V').trim().charAt(0).toUpperCase();

  return (
    <Screen edges={['top', 'left', 'right']} title="Profile" wash>
      <Card style={{ flexDirection: 'row', alignItems: 'center', gap: t.space.lg }}>
        <Gradient name="brand" style={{ width: 64, height: 64, borderRadius: 32, alignItems: 'center', justifyContent: 'center' }}>
          <Txt variant="title" tone="onBrand">
            {initial}
          </Txt>
        </Gradient>
        <View style={{ flex: 1, gap: t.space.xxs }}>
          <Txt variant="heading">{me?.name ?? 'Add your name'}</Txt>
          <Txt variant="small" tone="muted">
            {me?.phone ?? me?.email ?? ''}
          </Txt>
          <View
            style={{
              alignSelf: 'flex-start',
              marginTop: t.space.xs,
              backgroundColor: t.colors.accentSoft,
              borderRadius: t.radius.pill,
              paddingHorizontal: t.space.sm,
              paddingVertical: 2,
            }}
          >
            <Txt variant="caption" style={{ fontFamily: t.fonts.bold }}>
              {me?.plan === 'free' ? 'Free plan' : (me?.plan ?? '')}
            </Txt>
          </View>
        </View>
      </Card>

      <Card>
        <View style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'baseline' }}>
          <Txt variant="heading">Practice minutes</Txt>
          <Txt variant="bodyStrong" tone="primary">
            {formatMinutes(balance)} left
          </Txt>
        </View>
        <ProgressBar value={Math.min(1, balance / Math.max(FREE_SECS, balance))} />
        <Txt variant="small" tone="muted">
          Free drills today: {formatMinutes(me?.minutes.daily_drill_secs_left ?? 0)}. Plans and top-ups are coming soon.
        </Txt>
      </Card>

      <SectionTitle>Learning</SectionTitle>
      <ListGroup>
        <ListRow
          icon="language"
          title="Explanation language"
          value={languageLabel(me?.l1)}
          onPress={() => router.push('/settings/language')}
        />
        <ListRow
          icon="flag"
          title="Level & goal"
          subtitle={`${levelLabel(me?.level)} · ${goalLabel(me?.goal)}`}
          onPress={() => router.push('/settings/learning')}
        />
      </ListGroup>

      <SectionTitle>App</SectionTitle>
      <ListGroup>
        <ListRow icon="shield-checkmark" title="About & privacy" onPress={() => router.push('/settings/about')} />
        <ListRow icon="log-out" title="Sign out" tone="danger" onPress={signOut} />
      </ListGroup>
    </Screen>
  );
}
