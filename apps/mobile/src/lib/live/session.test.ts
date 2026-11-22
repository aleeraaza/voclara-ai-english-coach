import { bytesToBase64, floatToPcm16 } from './pcm';
import {
  LiveSession,
  type LiveResult,
  type MicLike,
  type PlayerLike,
  type SocketLike,
} from './session';

// The real Mic/Player import native audio; this test only uses fakes.
// (Jest hoists these above the imports.)
jest.mock('../audio/mic', () => ({ Mic: jest.fn() }));
jest.mock('../audio/player', () => ({ Player: jest.fn() }));

class FakeSocket implements SocketLike {
  binaryType?: string;
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: ((ev: { code: number; reason: string }) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  sent: any[] = [];
  closed = false;
  constructor(public url: string) {}
  send(data: string) {
    this.sent.push(JSON.parse(data));
  }
  close() {
    this.closed = true;
  }
  // test helpers
  open() {
    this.onopen?.();
  }
  server(msg: object) {
    this.onmessage?.({ data: JSON.stringify(msg) });
  }
  serverBinary(msg: object) {
    const bytes = new TextEncoder().encode(JSON.stringify(msg));
    this.onmessage?.({ data: bytes.buffer });
  }
  drop() {
    this.onclose?.({ code: 1006, reason: '' });
  }
}

class FakeMic implements MicLike {
  onChunk: ((s: Float32Array) => void) | null = null;
  started = 0;
  stopped = 0;
  async start(cb: (s: Float32Array) => void) {
    this.started++;
    this.onChunk = cb;
  }
  async stop() {
    this.stopped++;
    return 'file:///rec.m4a';
  }
  speak(n = 1600) {
    this.onChunk?.(new Float32Array(n).fill(0.1));
  }
}

class FakePlayer implements PlayerLike {
  played: number[] = [];
  interrupts = 0;
  closed = false;
  play(s: Float32Array) {
    this.played.push(s.length);
  }
  interrupt() {
    this.interrupts++;
  }
  async close() {
    this.closed = true;
  }
}

function setup(mode: 'push' | 'hands-free' = 'hands-free', minutes = 10) {
  let now = 0;
  const sockets: FakeSocket[] = [];
  const mic = new FakeMic();
  const player = new FakePlayer();
  const states: string[] = [];
  let result: LiveResult | null = null;
  const session = new LiveSession(
    { connectUrl: 'wss://g/ws', token: 'tok', model: 'live-x', endsAt: minutes * 60_000, mode },
    {
      socket: (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      mic,
      player,
      now: () => now,
    },
    { onState: (s) => states.push(s), onEnd: (r) => (result = r) },
  );
  return {
    session,
    sockets,
    mic,
    player,
    states,
    get result() {
      return result;
    },
    advance(ms: number) {
      now += ms;
      jest.advanceTimersByTime(ms);
    },
  };
}

const flush = () => new Promise((r) => jest.requireActual<typeof globalThis>('timers').setImmediate(r));
const audioSent = (s: FakeSocket) => s.sent.filter((m) => m.realtimeInput?.audio);

beforeEach(() => jest.useFakeTimers());
afterEach(() => jest.useRealTimers());

it('sends setup on open and starts the mic only after setupComplete', async () => {
  const t = setup();
  t.session.start();
  const ws = t.sockets[0]!;
  expect(ws.url).toBe('wss://g/ws?access_token=tok');
  ws.open();
  expect(ws.sent[0]).toEqual({ setup: { model: 'models/live-x' } });
  expect(t.mic.started).toBe(0);

  ws.server({ setupComplete: {} });
  await flush();
  expect(t.mic.started).toBe(1);
  expect(t.states).toContain('live');
});

it('hands-free streams every mic chunk as 16 kHz PCM', async () => {
  const t = setup('hands-free');
  t.session.start();
  const ws = t.sockets[0]!;
  ws.open();
  ws.server({ setupComplete: {} });
  await flush();
  t.mic.speak(1600);
  t.mic.speak(1600);
  const sent = audioSent(ws);
  expect(sent).toHaveLength(2);
  expect(sent[0].realtimeInput.audio.mimeType).toBe('audio/pcm;rate=16000');
  expect(sent[0].realtimeInput.audio.data).toBe(bytesToBase64(floatToPcm16(new Float32Array(1600).fill(0.1))));
});

it('push-to-talk sends only while held, then ends the audio stream', async () => {
  const t = setup('push');
  t.session.start();
  const ws = t.sockets[0]!;
  ws.open();
  ws.server({ setupComplete: {} });
  await flush();

  t.mic.speak();
  expect(audioSent(ws)).toHaveLength(0);

  t.session.setTalking(true);
  expect(t.player.interrupts).toBe(1); // pressing talk cuts the tutor off
  t.mic.speak();
  t.mic.speak();
  t.session.setTalking(false);
  t.mic.speak();

  expect(audioSent(ws)).toHaveLength(2);
  expect(ws.sent[ws.sent.length - 1]).toEqual({ realtimeInput: { audioStreamEnd: true } });
});

it('plays tutor audio, stops it on interruption and builds the transcript', async () => {
  const t = setup();
  t.session.start();
  const ws = t.sockets[0]!;
  ws.open();
  ws.server({ setupComplete: {} });
  await flush();

  const pcm = bytesToBase64(floatToPcm16(new Float32Array(2400)));
  ws.server({
    serverContent: {
      modelTurn: { parts: [{ inlineData: { mimeType: 'audio/pcm;rate=24000', data: pcm } }] },
      outputTranscription: { text: 'Tell me about your day.' },
    },
  });
  ws.serverBinary({ serverContent: { turnComplete: true } }); // binary frames work too
  ws.server({ serverContent: { inputTranscription: { text: 'I go to office.' } } });
  ws.server({ serverContent: { interrupted: true } });

  expect(t.player.played).toEqual([2400]);
  expect(t.player.interrupts).toBe(1);

  const result = await t.session.stop();
  expect(result.reason).toBe('user');
  expect(result.transcript.map((x) => [x.role, x.text])).toEqual([
    ['tutor', 'Tell me about your day.'],
    ['learner', 'I go to office.'],
  ]);
  expect(result.recordingPath).toBe('file:///rec.m4a');
  expect(ws.closed).toBe(true);
  expect(t.player.closed).toBe(true);
});

it('ends itself at the time limit and warns a minute before', async () => {
  const warnings: number[] = [];
  const s = new LiveSession(
    { connectUrl: 'wss://g/ws', token: 'tok', model: 'm', endsAt: 2 * 60_000, mode: 'hands-free' },
    { socket: (u) => new FakeSocket(u), mic: new FakeMic(), player: new FakePlayer(), now: () => 0 },
    { onTimeWarning: (ms) => warnings.push(ms), onEnd: () => undefined },
  );
  s.start();
  jest.advanceTimersByTime(60_000);
  expect(warnings).toEqual([60_000]);
  jest.advanceTimersByTime(60_000);
  await flush();
  const result = await s.stop(); // returns the already-finished result
  expect(result.reason).toBe('time_up');
});

it('resumes with the handle when the connection drops mid-session', async () => {
  const t = setup();
  t.session.start();
  const first = t.sockets[0]!;
  first.open();
  first.server({ setupComplete: {} });
  await flush();
  first.server({ sessionResumptionUpdate: { newHandle: 'h-1', resumable: true } });

  t.advance(30_000);
  first.drop();
  expect(t.states).toContain('reconnecting');
  const second = t.sockets[1]!;
  second.open();
  expect(second.sent[0]).toEqual({ setup: { model: 'models/live-x', sessionResumption: { handle: 'h-1' } } });

  // Audio is held back until the resumed session is ready.
  t.mic.speak();
  expect(audioSent(second)).toHaveLength(0);
  second.server({ setupComplete: {} });
  await flush();
  t.mic.speak();
  expect(audioSent(second)).toHaveLength(1);
  expect(t.mic.started).toBe(1); // the mic kept running across the reconnect
});

it('gives up as connection_lost when there is no resumption handle', async () => {
  const t = setup();
  t.session.start();
  const ws = t.sockets[0]!;
  ws.open();
  ws.server({ setupComplete: {} });
  await flush();
  ws.drop();
  await flush();
  expect(t.result?.reason).toBe('connection_lost');
  expect(t.sockets).toHaveLength(1);
});

it('reports an error when the socket closes before setup completes', async () => {
  const t = setup();
  t.session.start();
  t.sockets[0]!.drop();
  await flush();
  expect(t.result?.reason).toBe('error');
});

it('stop() is idempotent', async () => {
  const t = setup();
  t.session.start();
  const [a, b] = await Promise.all([t.session.stop(), t.session.stop()]);
  expect(a).toBe(b);
});
