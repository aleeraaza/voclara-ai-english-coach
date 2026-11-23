/** Band trend as vertical bars (oldest left). Built from Views: no chart package. */
import { View } from 'react-native';

import { useTheme } from '@/theme';

import { Txt } from './core';

export interface BandPoint {
  id: string;
  band: number;
  label: string;
  demo?: boolean;
}

export function BandChart({ points, height = 150 }: { points: BandPoint[]; height?: number }) {
  const t = useTheme();
  const shown = points.slice(-10);
  return (
    <View
      style={{ flexDirection: 'row', alignItems: 'flex-end', gap: t.space.sm, height: height + 40 }}
      accessibilityLabel={`Band trend: ${shown.map((p) => p.band.toFixed(1)).join(', ')}`}
    >
      {shown.map((p, i) => {
        const latest = i === shown.length - 1;
        return (
          <View key={p.id} style={{ flex: 1, alignItems: 'center', gap: t.space.xs }}>
            <Txt variant="caption" style={{ fontFamily: t.fonts.bold }} tone={latest ? 'primary' : 'muted'}>
              {p.band.toFixed(1)}
            </Txt>
            <View
              style={{
                width: '100%',
                maxWidth: 28,
                height: Math.max(8, (p.band / 9) * height),
                borderRadius: t.radius.sm,
                backgroundImage: p.demo ? undefined : latest ? t.gradients.sunrise : t.gradients.talk,
                backgroundColor: p.demo ? t.colors.surfaceAlt : undefined,
              }}
            />
            <Txt variant="caption" tone="muted" numberOfLines={1}>
              {p.label}
            </Txt>
          </View>
        );
      })}
    </View>
  );
}
