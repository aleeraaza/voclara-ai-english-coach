import { useState } from 'react';
import { Pressable, View } from 'react-native';

import { Gradient, Icon, Screen, SectionTitle, Txt, type IconName } from '@/components';
import { StartPanel } from '@/components/StartPanel';
import { useTheme } from '@/theme';

const ROLES: { value: string; label: string; icon: IconName; hint: string }[] = [
  { value: 'software', label: 'Software', icon: 'code-slash', hint: 'Projects, teamwork, problem solving' },
  { value: 'sales', label: 'Sales', icon: 'trending-up', hint: 'Targets, clients, persuasion' },
  { value: 'banking', label: 'Banking', icon: 'card', hint: 'Customers, accuracy, compliance' },
  { value: 'nursing', label: 'Nursing', icon: 'medkit', hint: 'Patient care, pressure, empathy' },
  { value: 'call_centre', label: 'Call centre', icon: 'headset', hint: 'Complaints, clarity, patience' },
];

export default function InterviewSetup() {
  const t = useTheme();
  const [role, setRole] = useState('software');

  return (
    <Screen>
      <Gradient name="interview" style={{ borderRadius: t.radius.xl, padding: t.space.xl, gap: t.space.xs }}>
        <Txt variant="title" tone="onBrand">
          Practise your interview
        </Txt>
        <Txt variant="small" tone="onBrandMuted">
          A friendly hiring manager asks 8–10 questions and follows up on what you say.
        </Txt>
      </Gradient>

      <SectionTitle>Choose the role</SectionTitle>
      <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: t.space.md }}>
        {ROLES.map((r) => {
          const on = r.value === role;
          return (
            <Pressable
              key={r.value}
              accessibilityRole="radio"
              accessibilityState={{ checked: on }}
              accessibilityLabel={`${r.label}. ${r.hint}`}
              onPress={() => setRole(r.value)}
              style={{
                width: '47%',
                flexGrow: 1,
                padding: t.space.lg,
                gap: t.space.sm,
                borderRadius: t.radius.lg,
                borderWidth: on ? 2 : 1,
                borderColor: on ? t.colors.info : t.colors.border,
                backgroundColor: on ? t.colors.infoSoft : t.colors.surface,
              }}
            >
              <View style={{ flexDirection: 'row', justifyContent: 'space-between' }}>
                <Icon name={r.icon} size={24} color={on ? t.colors.info : t.colors.textMuted} />
                {on && <Icon name="checkmark-circle" size={20} color={t.colors.info} />}
              </View>
              <Txt variant="bodyStrong">{r.label}</Txt>
              <Txt variant="caption" tone="muted">
                {r.hint}
              </Txt>
            </Pressable>
          );
        })}
      </View>

      <StartPanel mode="interview" topic={role} limitMins={15} />
    </Screen>
  );
}
