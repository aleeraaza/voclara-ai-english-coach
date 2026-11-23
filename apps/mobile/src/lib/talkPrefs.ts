import { useEffect, useState } from 'react';

import { kvGet, kvSet } from './kv';
import type { TalkMode } from './live/session';

const KEY = 'voclara.talkMode.v1';

/** Hold-to-talk vs hands-free, remembered between sessions. */
export function useTalkMode(): [TalkMode, (m: TalkMode) => void] {
  const [mode, setMode] = useState<TalkMode>('push');
  useEffect(() => {
    kvGet(KEY)
      .then((v) => {
        if (v === 'push' || v === 'hands-free') setMode(v);
      })
      .catch(() => null);
  }, []);
  const update = (m: TalkMode) => {
    setMode(m);
    kvSet(KEY, m).catch(() => null);
  };
  return [mode, update];
}
