import { router } from 'expo-router';
import { useState } from 'react';
import { Pressable, View } from 'react-native';

import { Button, ChipGroup, Field, Icon, Notice, Screen, SectionTitle, Txt } from '@/components';
import { api, errorMessage } from '@/lib/api/client';
import { useAuth } from '@/lib/auth';
import { GOALS, LEVELS } from '@/lib/labels';
import { useTheme } from '@/theme';

export default function LearningSettings() {
  const t = useTheme();
  const { me, refreshMe } = useAuth();
  const [name, setName] = useState(me?.name ?? '');
  const [level, setLevel] = useState<string>(me?.level ?? 'b1');
  const [goal, setGoal] = useState<string>(me?.goal ?? 'daily');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const levelHint = LEVELS.find((l) => l.value === level)?.hint;

  async function save() {
    setSaving(true);
    setError(null);
    const { error: e } = await api.PATCH('/api/v1/me', {
      body: { level, goal, name: name.trim() ? name.trim() : null },
    });
    setSaving(false);
    if (e) return setError(errorMessage(e));
    await refreshMe();
    router.back();
  }

  return (
    <Screen>
      <Field label="Your name" icon="person-outline" placeholder="e.g. Ayesha" value={name} onChangeText={setName} autoCapitalize="words" />

      <SectionTitle>Your English level</SectionTitle>
      <ChipGroup options={LEVELS} value={level} onChange={setLevel} />
      <Txt variant="small" tone="muted">
        {levelHint}. Sana matches her speed and vocabulary to this.
      </Txt>

      <SectionTitle>Your main goal</SectionTitle>
      <View style={{ gap: t.space.sm }}>
        {GOALS.map((g) => {
          const on = g.value === goal;
          return (
            <Pressable
              key={g.value}
              accessibilityRole="radio"
              accessibilityState={{ checked: on }}
              onPress={() => setGoal(g.value)}
              style={{
                flexDirection: 'row',
                alignItems: 'center',
                gap: t.space.md,
                minHeight: 56,
                paddingHorizontal: t.space.lg,
                borderRadius: t.radius.md,
                borderWidth: on ? 2 : 1,
                borderColor: on ? t.colors.primary : t.colors.border,
                backgroundColor: on ? t.colors.primarySoft : t.colors.surface,
              }}
            >
              <Icon name={g.icon} size={20} color={on ? t.colors.primary : t.colors.textMuted} />
              <Txt variant="bodyStrong" style={{ flex: 1 }}>
                {g.label}
              </Txt>
              {on && <Icon name="checkmark-circle" color={t.colors.primary} />}
            </Pressable>
          );
        })}
      </View>

      <Notice>{error}</Notice>
      <Button title="Save" size="lg" icon="checkmark" loading={saving} onPress={save} />
    </Screen>
  );
}
