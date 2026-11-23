import { Redirect } from 'expo-router';
import { useState } from 'react';
import { KeyboardAvoidingView, Platform, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { Button, Field, Notice, Txt, VoiceOrb } from '@/components';
import { errorMessage, publicApi } from '@/lib/api/client';
import { useAuth } from '@/lib/auth';
import { toE164 } from '@/lib/phone';
import { useTheme } from '@/theme';

const OFFLINE = { error: { message: 'Cannot reach Voclara. Check your internet.' } };

export default function SignIn() {
  const t = useTheme();
  const { signInWith, onboarded } = useAuth();
  const [phone, setPhone] = useState('');
  const [code, setCode] = useState('');
  const [step, setStep] = useState<'phone' | 'code'>('phone');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const e164 = toE164(phone);

  async function sendCode() {
    setBusy(true);
    setError(null);
    const { error: e } = await publicApi
      .POST('/api/v1/auth/otp/send', { body: { phone: e164 } })
      .catch(() => ({ error: OFFLINE }));
    setBusy(false);
    if (e) return setError(errorMessage(e));
    setStep('code');
  }

  async function verify() {
    setBusy(true);
    setError(null);
    const { data, error: e } = await publicApi
      .POST('/api/v1/auth/otp/verify', { body: { phone: e164, code: code.trim() } })
      .catch(() => ({ data: undefined, error: OFFLINE }));
    if (e || !data) {
      setBusy(false);
      return setError(errorMessage(e));
    }
    await signInWith(data);
  }

  // First launch: show the intro before asking for a number.
  if (onboarded === false) return <Redirect href="/onboarding" />;

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: t.colors.bg }}>
      <View pointerEvents="none" style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 340, backgroundImage: t.gradients.wash }} />
      <KeyboardAvoidingView behavior={Platform.OS === 'ios' ? 'padding' : undefined} style={{ flex: 1, padding: t.space.xxl, justifyContent: 'space-between' }}>
        <View style={{ alignItems: 'center', gap: t.space.sm, marginTop: t.space.xl }}>
          <VoiceOrb size={96} state="idle" icon="mic" />
          <Txt variant="title" align="center" accessibilityRole="header">
            {step === 'phone' ? 'Welcome to Voclara' : 'Enter your code'}
          </Txt>
          <Txt tone="muted" align="center">
            {step === 'phone'
              ? 'Sign in with your mobile number. No password needed.'
              : `We sent a 6-digit code to ${e164}.`}
          </Txt>
        </View>

        <View style={{ gap: t.space.lg }}>
          {step === 'phone' ? (
            <Field
              label="Mobile number"
              icon="call-outline"
              placeholder="0300 1234567"
              keyboardType="phone-pad"
              autoComplete="tel"
              value={phone}
              onChangeText={setPhone}
              onSubmitEditing={sendCode}
              hint="Pakistan numbers work as 03xx. Abroad? Start with +country code."
            />
          ) : (
            <Field
              label="6-digit code"
              icon="keypad-outline"
              placeholder="••••••"
              keyboardType="number-pad"
              autoComplete="one-time-code"
              textContentType="oneTimeCode"
              maxLength={6}
              value={code}
              onChangeText={setCode}
              onSubmitEditing={verify}
              style={{ letterSpacing: 8, fontSize: 22 }}
            />
          )}
          <Notice>{error}</Notice>
          {step === 'phone' ? (
            <Button
              title="Send code"
              size="lg"
              icon="paper-plane"
              loading={busy}
              disabled={phone.replace(/\D/g, '').length < 8}
              onPress={sendCode}
            />
          ) : (
            <>
              <Button title="Sign in" size="lg" icon="log-in" loading={busy} disabled={code.trim().length !== 6} onPress={verify} />
              <Button
                title="Use a different number"
                variant="ghost"
                onPress={() => {
                  setStep('phone');
                  setCode('');
                  setError(null);
                }}
              />
            </>
          )}
        </View>

        <Txt variant="caption" tone="muted" align="center">
          IELTS-style practice. Voclara is not affiliated with IELTS. By continuing you agree to our terms.
        </Txt>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}
