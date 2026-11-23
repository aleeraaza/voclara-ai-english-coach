import { Tabs } from 'expo-router';
import { View } from 'react-native';

import { Icon, type IconName } from '@/components';
import { useTheme } from '@/theme';

const TABS: { name: string; title: string; icon: IconName; iconActive: IconName }[] = [
  { name: 'index', title: 'Home', icon: 'home-outline', iconActive: 'home' },
  { name: 'practice', title: 'Practice', icon: 'mic-outline', iconActive: 'mic' },
  { name: 'progress', title: 'Progress', icon: 'stats-chart-outline', iconActive: 'stats-chart' },
  { name: 'profile', title: 'Profile', icon: 'person-circle-outline', iconActive: 'person-circle' },
];

export default function TabLayout() {
  const t = useTheme();
  return (
    <Tabs
      screenOptions={{
        headerShown: false,
        tabBarActiveTintColor: t.colors.primary,
        tabBarInactiveTintColor: t.colors.textMuted,
        tabBarStyle: {
          backgroundColor: t.colors.surface,
          borderTopColor: t.colors.border,
          height: 68,
          paddingTop: 6,
          boxShadow: t.shadows.raised,
        },
        tabBarLabelStyle: { fontFamily: t.fonts.semibold, fontSize: 12 },
      }}
    >
      {TABS.map((tab) => (
        <Tabs.Screen
          key={tab.name}
          name={tab.name}
          options={{
            title: tab.title,
            tabBarIcon: ({ focused, color }) => (
              <View
                style={{
                  width: 52,
                  height: 30,
                  borderRadius: t.radius.pill,
                  alignItems: 'center',
                  justifyContent: 'center',
                  backgroundColor: focused ? t.colors.primarySoft : 'transparent',
                }}
              >
                <Icon name={focused ? tab.iconActive : tab.icon} size={22} color={color as string} />
              </View>
            ),
          }}
        />
      ))}
    </Tabs>
  );
}
