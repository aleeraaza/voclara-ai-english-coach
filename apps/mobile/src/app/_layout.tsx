import {
  PlusJakartaSans_400Regular,
  PlusJakartaSans_500Medium,
  PlusJakartaSans_600SemiBold,
  PlusJakartaSans_700Bold,
  PlusJakartaSans_800ExtraBold,
  useFonts,
} from '@expo-google-fonts/plus-jakarta-sans';
import { Stack } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
import { useState } from 'react';
import { View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { Splash } from '@/components/Splash';
import { AuthProvider, useAuth } from '@/lib/auth';
import { useTheme } from '@/theme';

function RootStack() {
  const t = useTheme();
  const { status, onboarded } = useAuth();
  const [fontsLoaded, fontError] = useFonts({
    PlusJakartaSans_400Regular,
    PlusJakartaSans_500Medium,
    PlusJakartaSans_600SemiBold,
    PlusJakartaSans_700Bold,
    PlusJakartaSans_800ExtraBold,
  });
  // A font that fails to load falls back to the system font; never block the app.
  const fontsReady = fontsLoaded || !!fontError;
  const ready = fontsReady && status !== 'loading' && onboarded !== null;
  const [splashDone, setSplashDone] = useState(false);
  const signedIn = status === 'signedIn';

  return (
    <View style={{ flex: 1, backgroundColor: t.colors.bg }}>
      <StatusBar style={t.scheme === 'dark' ? 'light' : 'dark'} />
      {ready && (
        <Stack
          screenOptions={{
            headerShadowVisible: false,
            headerStyle: { backgroundColor: t.colors.bg },
            headerTintColor: t.colors.text,
            headerTitleStyle: { fontFamily: t.fonts.bold, fontSize: 17 },
            headerBackButtonDisplayMode: 'minimal',
            contentStyle: { backgroundColor: t.colors.bg },
            animation: 'slide_from_right',
          }}
        >
          <Stack.Protected guard={!signedIn && !onboarded}>
            <Stack.Screen name="onboarding" options={{ headerShown: false, animation: 'fade' }} />
          </Stack.Protected>
          <Stack.Protected guard={!signedIn}>
            <Stack.Screen name="sign-in" options={{ headerShown: false, animation: 'fade' }} />
          </Stack.Protected>
          <Stack.Protected guard={signedIn}>
            <Stack.Screen name="(tabs)" options={{ headerShown: false, animation: 'fade' }} />
            <Stack.Screen name="practice/talk" options={{ title: 'Talk with Sana' }} />
            <Stack.Screen name="practice/mock" options={{ title: 'IELTS-style mock' }} />
            <Stack.Screen name="practice/interview" options={{ title: 'Job interview' }} />
            <Stack.Screen
              name="talk"
              options={{ headerShown: false, gestureEnabled: false, animation: 'fade_from_bottom' }}
            />
            <Stack.Screen name="feedback" options={{ title: 'Your feedback', headerBackVisible: false, gestureEnabled: false }} />
            <Stack.Screen name="session/[id]" options={{ title: 'Session feedback' }} />
            <Stack.Screen name="settings/language" options={{ title: 'Explanation language' }} />
            <Stack.Screen name="settings/learning" options={{ title: 'You & your goal' }} />
            <Stack.Screen name="settings/about" options={{ title: 'About & privacy' }} />
          </Stack.Protected>
        </Stack>
      )}
      {!splashDone && <Splash ready={ready} fontsReady={fontsReady} onDone={() => setSplashDone(true)} />}
    </View>
  );
}

export default function RootLayout() {
  return (
    <SafeAreaProvider>
      <AuthProvider>
        <RootStack />
      </AuthProvider>
    </SafeAreaProvider>
  );
}
