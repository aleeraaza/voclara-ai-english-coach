/** Signed-in state for the whole app, plus the current user's profile. */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

import { api, publicApi, toPair, tokens, type Me, type Schemas } from './api/client';
import { hasOnboarded, markOnboarded } from './onboarding';

type Status = 'loading' | 'signedOut' | 'signedIn';

interface AuthValue {
  status: Status;
  /** null while loading. */
  onboarded: boolean | null;
  finishOnboarding: () => Promise<void>;
  me: Me | null;
  /** Reload the profile (minutes, plan, settings) from the API. */
  refreshMe: () => Promise<Me | null>;
  signInWith: (t: Schemas['TokenResponse']) => Promise<void>;
  signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<Status>('loading');
  const [onboarded, setOnboarded] = useState<boolean | null>(null);
  const [me, setMe] = useState<Me | null>(null);

  useEffect(() => {
    hasOnboarded()
      .then(setOnboarded)
      .catch(() => setOnboarded(true));
  }, []);

  const finishOnboarding = useCallback(async () => {
    setOnboarded(true);
    await markOnboarded().catch(() => null);
  }, []);

  const refreshMe = useCallback(async () => {
    const { data } = await api.GET('/api/v1/me');
    if (data) setMe(data);
    return data ?? null;
  }, []);

  useEffect(() => {
    const off = tokens.onChange((signedIn) => {
      setStatus(signedIn ? 'signedIn' : 'signedOut');
      if (!signedIn) setMe(null);
    });
    tokens
      .isSignedIn()
      .then(async (signedIn) => {
        setStatus(signedIn ? 'signedIn' : 'signedOut');
        if (signedIn) await refreshMe().catch(() => null);
      })
      .catch(() => setStatus('signedOut'));
    return off;
  }, [refreshMe]);

  const signInWith = useCallback(
    async (t: Schemas['TokenResponse']) => {
      await tokens.setTokens(toPair(t));
      await refreshMe().catch(() => null);
    },
    [refreshMe],
  );

  const signOut = useCallback(async () => {
    const refresh = await tokens.refreshToken();
    if (refresh) {
      // Best effort: revoke on the server, sign out locally regardless.
      await publicApi.POST('/api/v1/auth/logout', { body: { refresh_token: refresh } }).catch(() => null);
    }
    await tokens.signOut();
  }, []);

  const value = useMemo(
    () => ({ status, onboarded, finishOnboarding, me, refreshMe, signInWith, signOut }),
    [status, onboarded, finishOnboarding, me, refreshMe, signInWith, signOut],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthValue {
  const v = useContext(AuthContext);
  if (!v) throw new Error('useAuth must be used inside AuthProvider');
  return v;
}

export function useOnboarding() {
  const { onboarded, finishOnboarding } = useAuth();
  return { onboarded, finishOnboarding };
}
