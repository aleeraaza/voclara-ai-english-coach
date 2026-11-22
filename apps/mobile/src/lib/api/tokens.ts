/**
 * Access/refresh token handling.
 *
 * The API rotates refresh tokens and treats reuse of an old one as theft
 * (it signs the device out). So refreshing must be *single-flight*: if ten
 * requests notice an expired token at once, exactly one refresh call is made
 * and the other nine wait for its result.
 */

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  /** Epoch ms when the access token expires. */
  accessExpiresAt: number;
}

/** Persistent storage for the pair (SecureStore in the app, memory in tests). */
export interface TokenStorage {
  load(): Promise<TokenPair | null>;
  save(pair: TokenPair): Promise<void>;
  clear(): Promise<void>;
}

/** Calls POST /auth/refresh. Throws `RefreshRejected` on 401 (signed out). */
export type RefreshFn = (refreshToken: string) => Promise<TokenPair>;

export class RefreshRejected extends Error {
  constructor() {
    super('refresh token rejected');
    this.name = 'RefreshRejected';
  }
}

/** Refresh this long before expiry so a request never races the deadline. */
const EARLY_REFRESH_MS = 60_000;

export class TokenManager {
  private pair: TokenPair | null = null;
  private loaded = false;
  private inflight: Promise<TokenPair | null> | null = null;
  private listeners = new Set<(signedIn: boolean) => void>();

  constructor(
    private storage: TokenStorage,
    private refreshFn: RefreshFn,
    private now: () => number = Date.now,
  ) {}

  /** Notified with `false` when the session ends (sign-out or rejected refresh). */
  onChange(listener: (signedIn: boolean) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  async isSignedIn(): Promise<boolean> {
    await this.ensureLoaded();
    return this.pair !== null;
  }

  async setTokens(pair: TokenPair): Promise<void> {
    this.pair = pair;
    this.loaded = true;
    await this.storage.save(pair);
    this.emit(true);
  }

  async signOut(): Promise<void> {
    this.pair = null;
    this.loaded = true;
    await this.storage.clear();
    this.emit(false);
  }

  /** Current refresh token, e.g. for POST /auth/logout. */
  async refreshToken(): Promise<string | null> {
    await this.ensureLoaded();
    return this.pair?.refreshToken ?? null;
  }

  /**
   * A usable access token, refreshing first if it is about to expire.
   * `null` means signed out.
   */
  async getAccessToken(): Promise<string | null> {
    await this.ensureLoaded();
    if (!this.pair) return null;
    if (this.pair.accessExpiresAt - this.now() > EARLY_REFRESH_MS) {
      return this.pair.accessToken;
    }
    const refreshed = await this.refresh();
    return refreshed?.accessToken ?? null;
  }

  /**
   * Force a refresh (e.g. after a 401). Concurrent callers share one request.
   * Returns null if the server rejected the refresh token (signed out).
   */
  refresh(): Promise<TokenPair | null> {
    if (!this.inflight) {
      this.inflight = this.doRefresh().finally(() => {
        this.inflight = null;
      });
    }
    return this.inflight;
  }

  private async doRefresh(): Promise<TokenPair | null> {
    await this.ensureLoaded();
    const current = this.pair;
    if (!current) return null;
    try {
      const next = await this.refreshFn(current.refreshToken);
      this.pair = next;
      await this.storage.save(next);
      return next;
    } catch (err) {
      if (err instanceof RefreshRejected) {
        await this.signOut();
        return null;
      }
      // Network trouble: keep the tokens and let the caller retry later.
      throw err;
    }
  }

  private async ensureLoaded(): Promise<void> {
    if (this.loaded) return;
    this.pair = await this.storage.load();
    this.loaded = true;
  }

  private emit(signedIn: boolean) {
    for (const l of this.listeners) l(signedIn);
  }
}
