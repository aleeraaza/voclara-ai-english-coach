import { useState } from 'react';
import { View } from 'react-native';

import { Body, Button, Card, ErrorText, Field, Screen, Title } from '@/components/ui';
import { errorMessage, publicApi } from '@/lib/api/client';
import { useAuth } from '@/lib/auth';
import { toE164 } from '@/lib/phone';

export default function SignIn() {
  const { signInWith } = useAuth();
  const [phone, setPhone] = useState('');
  const [code, setCode] = useState('');
  const [step, setStep] = useState<'phone' | 'code'>('phone');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function sendCode() {
    setBusy(true);
    setError(null);
    const { error: e } = await publicApi
      .POST('/api/v1/auth/otp/send', { body: { phone: toE164(phone) } })
      .catch(() => ({ error: { error: { message: 'Cannot reach Voclara. Check your internet.' } } }));
    setBusy(false);
    if (e) return setError(errorMessage(e));
    setStep('code');
  }

  async function verify() {
    setBusy(true);
    setError(null);
    const { data, error: e } = await publicApi
      .POST('/api/v1/auth/otp/verify', { body: { phone: toE164(phone), code: code.trim() } })
      .catch(() => ({ data: undefined, error: { error: { message: 'Cannot reach Voclara. Check your internet.' } } }));
    if (e || !data) {
      setBusy(false);
      return setError(errorMessage(e));
    }
    await signInWith(data);
    // The router's protected stack moves to home once signed in.
  }

  return (
    <Screen>
      <View style={{ height: 48 }} />
      <Title>Speak English with confidence</Title>
      <Body muted>Practise with Sana, your AI speaking coach. IELTS-style mocks, interviews and daily talk.</Body>
      <Card>
        {step === 'phone' ? (
          <>
            <Field
              label="Mobile number"
              placeholder="0300 1234567"
              keyboardType="phone-pad"
              autoComplete="tel"
              value={phone}
              onChangeText={setPhone}
              onSubmitEditing={sendCode}
            />
            <Button title="Send code" onPress={sendCode} loading={busy} disabled={phone.replace(/\D/g, '').length < 8} />
          </>
        ) : (
          <>
            <Body muted>We sent a 6-digit code to {toE164(phone)}.</Body>
            <Field
              label="Code"
              placeholder="123456"
              keyboardType="number-pad"
              autoComplete="one-time-code"
              textContentType="oneTimeCode"
              maxLength={6}
              value={code}
              onChangeText={setCode}
              onSubmitEditing={verify}
            />
            <Button title="Sign in" onPress={verify} loading={busy} disabled={code.trim().length !== 6} />
            <Button
              title="Use a different number"
              variant="secondary"
              onPress={() => {
                setStep('phone');
                setCode('');
                setError(null);
              }}
            />
          </>
        )}
        <ErrorText>{error}</ErrorText>
      </Card>
      <Body muted style={{ fontSize: 13 }}>
        IELTS-style practice. Voclara is not affiliated with IELTS.
      </Body>
    </Screen>
  );
}
