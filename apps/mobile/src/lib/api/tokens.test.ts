import { RefreshRejected, TokenManager, type TokenPair, type TokenStorage } from './tokens';

class MemoryStorage implements TokenStorage {
  value: TokenPair | null = null;
  async load() {
    return this.value;
  }
  async save(pair: TokenPair) {
    this.value = pair;
  }
  async clear() {
    this.value = null;
  }
}

const pair = (n: number, expiresAt: number): TokenPair => ({
  accessToken: `access-${n}`,
  refreshToken: `refresh-${n}`,
  accessExpiresAt: expiresAt,
});

describe('TokenManager', () => {
  let now = 1_000_000;
  const clock = () => now;

  beforeEach(() => {
    now = 1_000_000;
  });

  it('returns the stored token while it is fresh', async () => {
    const storage = new MemoryStorage();
    storage.value = pair(1, now + 10 * 60_000);
    const refresh = jest.fn();
    const tm = new TokenManager(storage, refresh, clock);
    expect(await tm.getAccessToken()).toBe('access-1');
    expect(refresh).not.toHaveBeenCalled();
  });

  it('refreshes when the token is about to expire, and persists the new pair', async () => {
    const storage = new MemoryStorage();
    storage.value = pair(1, now + 30_000); // inside the 60 s early-refresh window
    const refresh = jest.fn(async () => pair(2, now + 15 * 60_000));
    const tm = new TokenManager(storage, refresh, clock);
    expect(await tm.getAccessToken()).toBe('access-2');
    expect(refresh).toHaveBeenCalledWith('refresh-1');
    expect(storage.value?.refreshToken).toBe('refresh-2');
  });

  it('makes exactly one refresh call for many concurrent requests', async () => {
    const storage = new MemoryStorage();
    storage.value = pair(1, now - 1);
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    const refresh = jest.fn(async () => {
      await gate;
      return pair(2, now + 15 * 60_000);
    });
    const tm = new TokenManager(storage, refresh, clock);

    const calls = Array.from({ length: 10 }, () => tm.getAccessToken());
    release();
    const tokens = await Promise.all(calls);

    expect(refresh).toHaveBeenCalledTimes(1);
    expect(new Set(tokens)).toEqual(new Set(['access-2']));
  });

  it('signs out when the server rejects the refresh token', async () => {
    const storage = new MemoryStorage();
    storage.value = pair(1, now - 1);
    const tm = new TokenManager(storage, async () => {
      throw new RefreshRejected();
    }, clock);
    const changes: boolean[] = [];
    tm.onChange((s) => changes.push(s));

    expect(await tm.getAccessToken()).toBeNull();
    expect(await tm.isSignedIn()).toBe(false);
    expect(storage.value).toBeNull();
    expect(changes).toEqual([false]);
  });

  it('keeps the tokens on a network error so a later retry can work', async () => {
    const storage = new MemoryStorage();
    storage.value = pair(1, now - 1);
    const refresh = jest
      .fn()
      .mockRejectedValueOnce(new TypeError('Network request failed'))
      .mockResolvedValueOnce(pair(2, now + 15 * 60_000));
    const tm = new TokenManager(storage, refresh, clock);

    await expect(tm.getAccessToken()).rejects.toThrow('Network request failed');
    expect(storage.value?.refreshToken).toBe('refresh-1');
    expect(await tm.getAccessToken()).toBe('access-2');
  });

  it('a second refresh after the first finished uses the new token', async () => {
    const storage = new MemoryStorage();
    storage.value = pair(1, now - 1);
    let n = 1;
    const refresh = jest.fn(async () => pair(++n, now + 15 * 60_000));
    const tm = new TokenManager(storage, refresh, clock);
    await tm.refresh();
    await tm.refresh();
    expect(refresh.mock.calls).toEqual([['refresh-1'], ['refresh-2']]);
  });

  it('is signed out with no stored tokens', async () => {
    const tm = new TokenManager(new MemoryStorage(), jest.fn(), clock);
    expect(await tm.isSignedIn()).toBe(false);
    expect(await tm.getAccessToken()).toBeNull();
  });
});
