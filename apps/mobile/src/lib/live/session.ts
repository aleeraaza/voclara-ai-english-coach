/**
 * One live conversation with Gemini, end to end:
 *
 *   connect -> send setup -> setupComplete -> mic on -> stream audio both ways
 *   -> stop (learner, time limit, or connection lost) -> transcript + recording
 *
 * Socket, mic and speaker are injected so this whole flow is unit-tested with
 * fakes; the real ones are wired in `createLiveSession` at the bottom.
 */
import { Mic } from '../audio/mic';
import { Player } from '../audio/player';
import {
  audioMessage,
  audioStreamEndMessage,
  connectUrl,
  parseServerMessage,
  setupMessage,
  OUTPUT_SAMPLE_RATE,
} from './gemini';
import { base64ToBytes, bytesToBase64, floatToPcm16, pcm16ToFloat, rms, utf8Decode } from './pcm';
import { TranscriptBuilder, type Turn } from './transcript';

export interface MicLike {
  start(onChunk: (samples: Float32Array) => void): Promise<void>;
  stop(): Promise<string | null>;
}

export interface PlayerLike {
  play(samples: Float32Array, sampleRate: number): void;
  interrupt(): void;
  close(): Promise<void>;
}

export interface SocketLike {
  binaryType?: string;
  onopen: (() => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onclose: ((ev: { code: number; reason: string }) => void) | null;
  onerror: ((ev: unknown) => void) | null;
  send(data: string): void;
  close(): void;
}

export type SocketFactory = (url: string) => SocketLike;

export type TalkMode = 'push' | 'hands-free';
export type LiveState = 'connecting' | 'live' | 'reconnecting' | 'ended';
export type EndReason = 'user' | 'time_up' | 'connection_lost' | 'error';

export interface LiveResult {
  reason: EndReason;
  transcript: Turn[];
  recordingPath: string | null;
  durationMs: number;
  error?: string;
}

export interface LiveCallbacks {
  onState?: (state: LiveState) => void;
  onTranscript?: (turns: Turn[]) => void;
  onTutorSpeaking?: (speaking: boolean) => void;
  onLevel?: (level: number) => void;
  /** Fired once, about a minute before the time limit. */
  onTimeWarning?: (msLeft: number) => void;
  onEnd?: (result: LiveResult) => void;
}

export interface LiveOptions {
  connectUrl: string;
  token: string;
  model: string;
  /** Epoch ms: the session's hard end (`session_ends_at`). */
  endsAt: number;
  mode: TalkMode;
}

export interface LiveDeps {
  socket: SocketFactory;
  mic: MicLike;
  player: PlayerLike;
  now?: () => number;
}

const MAX_RECONNECTS = 3;
const WARNING_MS = 60_000;

export class LiveSession {
  private ws: SocketLike | null = null;
  private state: LiveState = 'connecting';
  private ready = false; // setupComplete received on the current socket
  private micOn = false;
  private talking = false; // push-to-talk button held
  private resumeHandle: string | null = null;
  private reconnects = 0;
  private startedAt: number;
  private transcript: TranscriptBuilder;
  private endTimer: ReturnType<typeof setTimeout> | null = null;
  private warnTimer: ReturnType<typeof setTimeout> | null = null;
  private finishing: Promise<LiveResult> | null = null;
  private now: () => number;

  constructor(
    private opts: LiveOptions,
    private deps: LiveDeps,
    private cb: LiveCallbacks = {},
  ) {
    this.now = deps.now ?? Date.now;
    this.startedAt = this.now();
    this.transcript = new TranscriptBuilder(this.startedAt, this.now);
  }

  get mode(): TalkMode {
    return this.opts.mode;
  }

  start(): void {
    const msLeft = this.opts.endsAt - this.now();
    if (msLeft <= 0) {
      void this.finish('time_up');
      return;
    }
    this.endTimer = setTimeout(() => void this.finish('time_up'), msLeft);
    if (msLeft > WARNING_MS) {
      this.warnTimer = setTimeout(() => this.cb.onTimeWarning?.(WARNING_MS), msLeft - WARNING_MS);
    }
    this.connect();
  }

  /** Push-to-talk: hold to speak. Pressing also cuts the tutor off. */
  setTalking(down: boolean): void {
    if (this.opts.mode !== 'push' || this.talking === down) return;
    this.talking = down;
    if (down) {
      this.deps.player.interrupt();
    } else if (this.ready) {
      this.ws?.send(audioStreamEndMessage());
    }
  }

  setMode(mode: TalkMode): void {
    if (mode === this.opts.mode) return;
    if (this.opts.mode === 'push' && this.talking && this.ready) this.ws?.send(audioStreamEndMessage());
    this.talking = false;
    this.opts = { ...this.opts, mode };
  }

  /** End the session (button). Safe to call more than once. */
  stop(): Promise<LiveResult> {
    return this.finish('user');
  }

  private connect(): void {
    const ws = this.deps.socket(connectUrl(this.opts.connectUrl, this.opts.token));
    ws.binaryType = 'arraybuffer';
    this.ws = ws;
    this.ready = false;
    ws.onopen = () => ws.send(setupMessage(this.opts.model, this.resumeHandle ?? undefined));
    ws.onmessage = (ev) => this.onMessage(ev.data);
    ws.onerror = () => {
      // An error is always followed by close; handle it there.
    };
    ws.onclose = () => {
      if (this.ws !== ws || this.state === 'ended') return;
      this.onConnectionLost();
    };
  }

  private onConnectionLost(): void {
    this.ready = false;
    const timeLeft = this.opts.endsAt - this.now();
    if (this.resumeHandle && timeLeft > 5_000 && this.reconnects < MAX_RECONNECTS) {
      this.reconnects += 1;
      this.setState('reconnecting');
      this.connect();
      return;
    }
    void this.finish(this.state === 'connecting' ? 'error' : 'connection_lost');
  }

  private onMessage(data: unknown): void {
    const text =
      typeof data === 'string'
        ? data
        : data instanceof ArrayBuffer
          ? utf8Decode(new Uint8Array(data))
          : '';
    for (const ev of parseServerMessage(text)) {
      switch (ev.type) {
        case 'setupComplete':
          this.ready = true;
          this.reconnects = 0;
          this.setState('live');
          void this.ensureMic();
          break;
        case 'audio':
          this.deps.player.play(pcm16ToFloat(base64ToBytes(ev.data)), ev.sampleRate || OUTPUT_SAMPLE_RATE);
          break;
        case 'interrupted':
          this.deps.player.interrupt();
          break;
        case 'learnerText':
          this.transcript.add('learner', ev.text);
          this.cb.onTranscript?.(this.transcript.toJSON());
          break;
        case 'tutorText':
          this.transcript.add('tutor', ev.text);
          this.cb.onTranscript?.(this.transcript.toJSON());
          break;
        case 'turnComplete':
          this.transcript.closeTurn();
          break;
        case 'resumption':
          this.resumeHandle = ev.handle;
          break;
        case 'goAway':
          // The server will drop us soon; the close handler resumes with the handle.
          break;
      }
    }
  }

  private async ensureMic(): Promise<void> {
    if (this.micOn) return;
    this.micOn = true;
    try {
      await this.deps.mic.start((samples) => this.onMicChunk(samples));
    } catch (e) {
      this.micOn = false;
      void this.finish('error', e instanceof Error ? e.message : String(e));
    }
  }

  private onMicChunk(samples: Float32Array): void {
    this.cb.onLevel?.(rms(samples));
    if (!this.ready || this.state !== 'live') return;
    if (this.opts.mode === 'push' && !this.talking) return;
    this.ws?.send(audioMessage(bytesToBase64(floatToPcm16(samples))));
  }

  private finish(reason: EndReason, error?: string): Promise<LiveResult> {
    if (!this.finishing) {
      this.finishing = this.doFinish(reason, error);
    }
    return this.finishing;
  }

  private async doFinish(reason: EndReason, error?: string): Promise<LiveResult> {
    this.setState('ended');
    if (this.endTimer) clearTimeout(this.endTimer);
    if (this.warnTimer) clearTimeout(this.warnTimer);
    const ws = this.ws;
    this.ws = null;
    try {
      ws?.close();
    } catch {
      // Already closed.
    }
    let recordingPath: string | null = null;
    if (this.micOn) {
      this.micOn = false;
      recordingPath = await this.deps.mic.stop().catch(() => null);
    }
    await this.deps.player.close().catch(() => undefined);
    const result: LiveResult = {
      reason,
      transcript: this.transcript.toJSON(),
      recordingPath,
      durationMs: this.now() - this.startedAt,
      error,
    };
    this.cb.onEnd?.(result);
    return result;
  }

  private setState(s: LiveState): void {
    if (this.state === s) return;
    this.state = s;
    this.cb.onState?.(s);
  }
}

/** Real session with the device mic, speaker and WebSocket. */
export function createLiveSession(opts: LiveOptions, cb: LiveCallbacks): LiveSession {
  return new LiveSession(
    opts,
    {
      socket: (url) => new WebSocket(url) as unknown as SocketLike,
      mic: new Mic(),
      player: new Player((speaking) => cb.onTutorSpeaking?.(speaking)),
    },
    cb,
  );
}
