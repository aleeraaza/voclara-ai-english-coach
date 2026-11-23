import { useState } from 'react';
import { Pressable, View } from 'react-native';

import { Icon, Notice, Screen, Txt } from '@/components';
import { api, errorMessage } from '@/lib/api/client';
import { useAuth } from '@/lib/auth';
import { LANGUAGES } from '@/lib/labels';
import { useTheme } from '@/theme';

export default function LanguageSettings() {
  const t = useTheme();
  const { me, refreshMe } = useAuth();
  const [selected, setSelected] = useState<string>(me?.l1 ?? 'ur');
  const [error, setError] = useState<string | null>(null);

  async function choose(value: string) {
    const previous = selected;
    setSelected(value);
    setError(null);
    const { error: e } = await api.PATCH('/api/v1/me', { body: { l1: value } });
    if (e) {
      setSelected(previous);
      return setError(errorMessage(e));
    }
    await refreshMe();
  }

  return (
    <Screen>
      <Txt tone="muted">Corrections are explained in English and in this language.</Txt>
      <View style={{ gap: t.space.sm }}>
        {LANGUAGES.map((l) => {
          const on = l.value === selected;
          return (
            <Pressable
              key={l.value}
              accessibilityRole="radio"
              accessibilityState={{ checked: on }}
              onPress={() => choose(l.value)}
              style={({ pressed }) => ({
                flexDirection: 'row',
                alignItems: 'center',
                gap: t.space.md,
                minHeight: 64,
                paddingHorizontal: t.space.lg,
                borderRadius: t.radius.lg,
                borderWidth: on ? 2 : 1,
                borderColor: on ? t.colors.primary : t.colors.border,
                backgroundColor: on ? t.colors.primarySoft : pressed ? t.colors.surfaceAlt : t.colors.surface,
              })}
            >
              <View style={{ flex: 1 }}>
                <Txt variant="bodyStrong">{l.label}</Txt>
              </View>
              <Txt variant="heading" style={{ fontFamily: undefined }}>
                {l.native}
              </Txt>
              <Icon name={on ? 'radio-button-on' : 'radio-button-off'} color={on ? t.colors.primary : t.colors.textMuted} />
            </Pressable>
          );
        })}
      </View>
      <Notice>{error}</Notice>
    </Screen>
  );
}
