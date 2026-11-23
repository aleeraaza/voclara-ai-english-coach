import { kvGet, kvSet } from './kv';

const KEY = 'voclara.onboarded.v1';

export async function hasOnboarded(): Promise<boolean> {
  return (await kvGet(KEY)) === '1';
}

export async function markOnboarded(): Promise<void> {
  await kvSet(KEY, '1');
}
