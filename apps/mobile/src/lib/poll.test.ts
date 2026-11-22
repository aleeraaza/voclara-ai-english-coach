import { pollUntil } from './poll';

function fakeClock() {
  let t = 0;
  const sleeps: number[] = [];
  return {
    now: () => t,
    sleep: async (ms: number) => {
      sleeps.push(ms);
      t += ms;
    },
    sleeps,
  };
}

describe('pollUntil', () => {
  it('stops as soon as the value is final, backing off in between', async () => {
    const clock = fakeClock();
    const values = ['pending', 'pending', 'pending', 'ready'];
    const seen: string[] = [];
    const result = await pollUntil(async () => values.shift()!, {
      done: (v) => v !== 'pending',
      onValue: (v) => seen.push(v),
      intervalMs: 1000,
      maxIntervalMs: 3000,
      ...clock,
    });
    expect(result).toBe('ready');
    expect(seen).toEqual(['pending', 'pending', 'pending', 'ready']);
    expect(clock.sleeps).toEqual([1000, 2000, 3000]);
  });

  it('gives up after the timeout and returns the last value', async () => {
    const clock = fakeClock();
    const fetch = jest.fn(async () => 'pending');
    const result = await pollUntil(fetch, { done: () => false, intervalMs: 1000, timeoutMs: 5000, ...clock });
    expect(result).toBe('pending');
    expect(clock.now()).toBeGreaterThanOrEqual(5000);
    expect(fetch.mock.calls.length).toBeLessThan(6);
  });

  it('stops when cancelled', async () => {
    const clock = fakeClock();
    let cancelled = false;
    const fetch = jest.fn(async () => {
      cancelled = true;
      return 'pending';
    });
    await pollUntil(fetch, { done: () => false, cancelled: () => cancelled, ...clock });
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});
