/**
 * Typed API client. Types come from the Rust API's OpenAPI spec
 * (packages/api-types, regenerate with `npm run gen:api`), so a change in the
 * API shows up here as a type error.
 */
import createClient from 'openapi-fetch';
import * as SecureStore from 'expo-secure-store';
import type { components, paths } from '@api-types';

import { RefreshRejected, TokenManager, type TokenPair, type TokenStorage } from './tokens';

export type Schemas = components['schemas'];
export type Me = Schemas['MeResponse'];
export type StartedSession = Schemas['StartSessionResponse'];

/**
 * API base URL. Set EXPO_PUBLIC_API_URL in apps/mobile/.env.local.
 * Default: the Android emulator's alias for the host machine's localhost.
 */
export const API_URL = (process.env.EXPO_PUBLIC_API_URL ?? 'http://10.0.2.2:8080').replace(/\/$/, '');

const STORAGE_KEY = 'voclara.tokens.v1';

const secureStorage: TokenStorage = {
  async load() {
    const raw = await SecureStore.getItemAsync(STORAGE_KEY);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as TokenPair;
    } catch {
      return null;
    }
  },
  async save(pair) {
    await SecureStore.setItemAsync(STORAGE_KEY, JSON.stringify(pair));
  },
  async clear() {
    await SecureStore.deleteItemAsync(STORAGE_KEY);
  },
};

export function toPair(t: Schemas['TokenResponse'], now = Date.now()): TokenPair {
  return {
    accessToken: t.access_token,
    refreshToken: t.refresh_token,
    accessExpiresAt: now + t.expires_in * 1000,
  };
}

/** Unauthenticated client, used for sign-in and refresh. */
export const publicApi = createClient<paths>({ baseUrl: API_URL });

async function refreshFn(refreshToken: string): Promise<TokenPair> {
  const { data, response } = await publicApi.POST('/api/v1/auth/refresh', {
    body: { refresh_token: refreshToken },
  });
  if (response.status === 401) throw new RefreshRejected();
  if (!data) throw new Error(`refresh failed (${response.status})`);
  return toPair(data);
}

export const tokens = new TokenManager(secureStorage, refreshFn);

/**
 * fetch with the bearer token attached. On a 401 it refreshes once and
 * retries once; if that also fails the user is signed out.
 */
async function authedFetch(request: Request): Promise<Response> {
  const retry = request.clone(); // a body can only be read once
  const token = await tokens.getAccessToken();
  if (!token) return new Response(null, { status: 401 });
  request.headers.set('Authorization', `Bearer ${token}`);
  const first = await fetch(request);
  if (first.status !== 401) return first;

  const refreshed = await tokens.refresh();
  if (!refreshed) return first;
  retry.headers.set('Authorization', `Bearer ${refreshed.accessToken}`);
  return fetch(retry);
}

/** Authenticated client for everything under /api/v1 except sign-in. */
export const api = createClient<paths>({ baseUrl: API_URL, fetch: authedFetch });

/** Human-readable message from the API's `{error: {code, message}}` body. */
export function errorMessage(error: unknown, fallback = 'Something went wrong. Please try again.'): string {
  const e = error as { error?: { message?: string } } | undefined;
  return e?.error?.message ?? fallback;
}

export function errorCode(error: unknown): string | undefined {
  return (error as { error?: { code?: string } } | undefined)?.error?.code;
}
