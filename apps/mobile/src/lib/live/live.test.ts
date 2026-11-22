import {
  audioMessage,
  audioStreamEndMessage,
  connectUrl,
  parseDuration,
  parseServerMessage,
  setupMessage,
} from './gemini';
import {
  base64ToBytes,
  bytesToBase64,
  floatToPcm16,
  pcm16ToFloat,
  resample,
  rms,
  utf8Decode,
} from './pcm';
import { TranscriptBuilder, joinFragments } from './transcript';

describe('base64', () => {
  it('round-trips every padding length', () => {
    for (const len of [0, 1, 2, 3, 4, 5, 255, 1000]) {
      const bytes = Uint8Array.from({ length: len }, (_, i) => (i * 37) % 256);
      expect(Array.from(base64ToBytes(bytesToBase64(bytes)))).toEqual(Array.from(bytes));
    }
  });

  it('matches the standard alphabet', () => {
    const hello = new Uint8Array([72, 101, 108, 108, 111]);
    expect(bytesToBase64(hello)).toBe('SGVsbG8=');
    expect(Array.from(base64ToBytes('SGVsbG8='))).toEqual([72, 101, 108, 108, 111]);
  });
});

describe('PCM16', () => {
  it('encodes little-endian and clips out-of-range samples', () => {
    const bytes = floatToPcm16(new Float32Array([0, 1, -1, 2, -2]));
    const view = new DataView(bytes.buffer);
    expect(view.getInt16(0, true)).toBe(0);
    expect(view.getInt16(2, true)).toBe(32767);
    expect(view.getInt16(4, true)).toBe(-32768);
    expect(view.getInt16(6, true)).toBe(32767);
    expect(view.getInt16(8, true)).toBe(-32768);
  });

  it('round-trips within one quantisation step', () => {
    const src = new Float32Array([0, 0.5, -0.5, 0.25, -0.999]);
    const back = pcm16ToFloat(floatToPcm16(src));
    src.forEach((v, i) => expect(Math.abs(back[i]! - v)).toBeLessThan(1 / 32767));
  });

  it('ignores a trailing odd byte', () => {
    expect(pcm16ToFloat(new Uint8Array([0, 0, 1])).length).toBe(1);
  });

  it('resamples 48 kHz to 16 kHz by length and keeps a constant signal', () => {
    const out = resample(new Float32Array(4800).fill(0.5), 48_000, 16_000);
    expect(out.length).toBe(1600);
    expect(out.every((v) => Math.abs(v - 0.5) < 1e-6)).toBe(true);
    const same = new Float32Array([1, 2]);
    expect(resample(same, 16_000, 16_000)).toBe(same);
  });

  it('rms of silence is 0 and of a full-scale square wave is 1', () => {
    expect(rms(new Float32Array(10))).toBe(0);
    expect(rms(new Float32Array([1, -1, 1, -1]))).toBe(1);
  });
});

describe('utf8Decode', () => {
  it('decodes multi-byte text', () => {
    const bytes = new Uint8Array([0x7b, 0x22, 0x78, 0x22, 0x3a, 0x22, 0xd8, 0xa7, 0xe2, 0x9c, 0x93, 0x22, 0x7d]);
    expect(JSON.parse(utf8Decode(bytes))).toEqual({ x: 'ا✓' });
  });
});

describe('Gemini client messages', () => {
  it('puts the token in the URL', () => {
    expect(connectUrl('wss://h/ws/X', 'auth_tokens/a b')).toBe(
      'wss://h/ws/X?access_token=auth_tokens%2Fa%20b',
    );
  });

  it('setup names the model and optionally resumes', () => {
    expect(JSON.parse(setupMessage('live-x'))).toEqual({ setup: { model: 'models/live-x' } });
    expect(JSON.parse(setupMessage('models/live-x', 'h1'))).toEqual({
      setup: { model: 'models/live-x', sessionResumption: { handle: 'h1' } },
    });
  });

  it('audio goes as 16 kHz PCM realtime input', () => {
    expect(JSON.parse(audioMessage('AAA='))).toEqual({
      realtimeInput: { audio: { data: 'AAA=', mimeType: 'audio/pcm;rate=16000' } },
    });
    expect(JSON.parse(audioStreamEndMessage())).toEqual({ realtimeInput: { audioStreamEnd: true } });
  });
});

describe('parseServerMessage', () => {
  it('reads setupComplete', () => {
    expect(parseServerMessage('{"setupComplete":{}}')).toEqual([{ type: 'setupComplete' }]);
  });

  it('reads audio, both captions and turnComplete from one message', () => {
    const msg = {
      serverContent: {
        modelTurn: { parts: [{ inlineData: { mimeType: 'audio/pcm;rate=24000', data: 'AQI=' } }] },
        inputTranscription: { text: 'I go to office' },
        outputTranscription: { text: 'You mean: I went' },
        turnComplete: true,
      },
    };
    expect(parseServerMessage(JSON.stringify(msg))).toEqual([
      { type: 'audio', data: 'AQI=', sampleRate: 24000 },
      { type: 'learnerText', text: 'I go to office' },
      { type: 'tutorText', text: 'You mean: I went' },
      { type: 'turnComplete' },
    ]);
  });

  it('reports interruption before anything else in the message', () => {
    const events = parseServerMessage(JSON.stringify({ serverContent: { interrupted: true, turnComplete: true } }));
    expect(events.map((e) => e.type)).toEqual(['interrupted', 'turnComplete']);
  });

  it('reads goAway and resumption handles', () => {
    expect(parseServerMessage('{"goAway":{"timeLeft":"9.5s"}}')).toEqual([{ type: 'goAway', timeLeftMs: 9500 }]);
    expect(
      parseServerMessage('{"sessionResumptionUpdate":{"newHandle":"h2","resumable":true}}'),
    ).toEqual([{ type: 'resumption', handle: 'h2' }]);
    expect(parseServerMessage('{"sessionResumptionUpdate":{"resumable":false}}')).toEqual([]);
  });

  it('ignores garbage and unknown fields', () => {
    expect(parseServerMessage('not json')).toEqual([]);
    expect(parseServerMessage('{"usageMetadata":{"totalTokenCount":5}}')).toEqual([]);
    expect(parseServerMessage('{"serverContent":{"modelTurn":{"parts":[{"text":"hi"}]}}}')).toEqual([]);
  });

  it('parses durations', () => {
    expect(parseDuration('3s')).toBe(3000);
    expect(parseDuration('0.25s')).toBe(250);
    expect(parseDuration(5)).toBeNull();
  });
});

describe('TranscriptBuilder', () => {
  it('joins fragments per speaker and timestamps each turn', () => {
    let now = 1000;
    const t = new TranscriptBuilder(1000, () => now);
    t.add('tutor', 'Hi! Tell me');
    t.add('tutor', 'about your day.');
    t.closeTurn();
    now = 4200;
    t.add('learner', 'I go');
    t.add('learner', ' to office yesterday');
    t.add('learner', '.');
    now = 7000;
    t.add('tutor', 'You mean: I went.');
    expect(t.toJSON()).toEqual([
      { role: 'tutor', text: 'Hi! Tell me about your day.', t_ms: 0 },
      { role: 'learner', text: 'I go to office yesterday.', t_ms: 3200 },
      { role: 'tutor', text: 'You mean: I went.', t_ms: 6000 },
    ]);
  });

  it('starts a new tutor turn after turnComplete even with no learner speech', () => {
    const t = new TranscriptBuilder(0, () => 0);
    t.add('tutor', 'First.');
    t.closeTurn();
    t.add('tutor', 'Second.');
    expect(t.toJSON().map((x) => x.text)).toEqual(['First.', 'Second.']);
  });

  it('drops empty turns', () => {
    const t = new TranscriptBuilder(0, () => 0);
    t.add('learner', '   ');
    expect(t.toJSON()).toEqual([]);
  });

  it('joinFragments only adds missing spaces', () => {
    expect(joinFragments('Hello', 'world')).toBe('Hello world');
    expect(joinFragments('Hello ', 'world')).toBe('Hello world');
    expect(joinFragments('Hello', ', world')).toBe('Hello, world');
  });
});
