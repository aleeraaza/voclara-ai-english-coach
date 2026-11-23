import { useFocusEffect } from 'expo-router';
import { useCallback, useState } from 'react';

import { api, type Progress } from './api/client';

/** Band history, refreshed whenever the screen is focused. */
export function useProgress(): Progress | null {
  const [progress, setProgress] = useState<Progress | null>(null);
  useFocusEffect(
    useCallback(() => {
      let alive = true;
      api
        .GET('/api/v1/progress')
        .then(({ data }) => {
          if (alive && data) setProgress(data);
        })
        .catch(() => null);
      return () => {
        alive = false;
      };
    }, []),
  );
  return progress;
}
