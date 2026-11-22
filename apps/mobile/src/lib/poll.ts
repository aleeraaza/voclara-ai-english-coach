/**
 * Poll an async source until it reports a final state or time runs out.
 * Used for session feedback, which is graded in the background.
 */
export interface PollOptions<T> {
  /** True when `value` is final (stop polling). */
  done: (value: T) => boolean;
  /** Wait before the first retry; doubles each time up to `maxIntervalMs`. */
  intervalMs?: number;
  maxIntervalMs?: number;
  /** Give up after this long and return the last value. */
  timeoutMs?: number;
  /** Called with every value, so the screen can update while waiting. */
  onValue?: (value: T) => void;
  /** Stop early (screen closed). */
  cancelled?: () => boolean;
  sleep?: (ms: number) => Promise<void>;
  now?: () => number;
}

const defaultSleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

export async function pollUntil<T>(fetch: () => Promise<T>, opts: PollOptions<T>): Promise<T> {
  const {
    done,
    intervalMs = 2_000,
    maxIntervalMs = 8_000,
    timeoutMs = 120_000,
    onValue,
    cancelled = () => false,
    sleep = defaultSleep,
    now = Date.now,
  } = opts;
  const started = now();
  let wait = intervalMs;
  for (;;) {
    const value = await fetch();
    onValue?.(value);
    if (done(value) || cancelled() || now() - started >= timeoutMs) return value;
    await sleep(wait);
    if (cancelled()) return value;
    wait = Math.min(wait * 2, maxIntervalMs);
  }
}
