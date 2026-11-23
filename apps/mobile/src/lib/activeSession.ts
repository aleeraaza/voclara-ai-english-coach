/**
 * Remembers the session that is running, so that if the app is killed
 * mid-session the next start can close it first (the API allows one active
 * session per user and answers 409 otherwise).
 */
import { kvDelete, kvGet, kvSet } from './kv';

const KEY = 'voclara.activeSession.v1';

export async function rememberActiveSession(id: string): Promise<void> {
  await kvSet(KEY, id);
}

export async function activeSessionId(): Promise<string | null> {
  return kvGet(KEY);
}

export async function forgetActiveSession(): Promise<void> {
  await kvDelete(KEY);
}
