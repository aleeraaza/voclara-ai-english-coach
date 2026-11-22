/**
 * Gemini Live WebSocket protocol (client side), as documented at
 * https://ai.google.dev/api/live. Only message shapes live here; the socket,
 * mic and speaker are wired up in session.ts.
 *
 * The session setup (prompt, voice, transcription) is locked into the
 * ephemeral token by our API, so the client's `setup` only names the model.
 */

export const INPUT_SAMPLE_RATE = 16_000;
export const OUTPUT_SAMPLE_RATE = 24_000;
const INPUT_MIME = `audio/pcm;rate=${INPUT_SAMPLE_RATE}`;

export function connectUrl(base: string, token: string): string {
  const sep = base.includes('?') ? '&' : '?';
  return `${base}${sep}access_token=${encodeURIComponent(token)}`;
}

export function setupMessage(model: string, resumeHandle?: string): string {
  const name = model.startsWith('models/') ? model : `models/${model}`;
  const setup: Record<string, unknown> = { model: name };
  if (resumeHandle) setup.sessionResumption = { handle: resumeHandle };
  return JSON.stringify({ setup });
}

export function audioMessage(pcm16Base64: string): string {
  return JSON.stringify({ realtimeInput: { audio: { data: pcm16Base64, mimeType: INPUT_MIME } } });
}

/** Tell Gemini the mic is closed (push-to-talk released), so it answers now. */
export function audioStreamEndMessage(): string {
  return JSON.stringify({ realtimeInput: { audioStreamEnd: true } });
}

export type ServerEvent =
  | { type: 'setupComplete' }
  | { type: 'audio'; data: string; sampleRate: number }
  | { type: 'learnerText'; text: string }
  | { type: 'tutorText'; text: string }
  | { type: 'interrupted' }
  | { type: 'turnComplete' }
  | { type: 'goAway'; timeLeftMs: number | null }
  | { type: 'resumption'; handle: string };

/** "12.5s" (protobuf Duration JSON) -> ms. */
export function parseDuration(d: unknown): number | null {
  if (typeof d !== 'string') return null;
  const m = /^(-?\d+(?:\.\d+)?)s$/.exec(d.trim());
  return m ? Math.round(parseFloat(m[1]!) * 1000) : null;
}

function rateFromMime(mime: unknown): number {
  const m = typeof mime === 'string' ? /rate=(\d+)/.exec(mime) : null;
  return m ? parseInt(m[1]!, 10) : OUTPUT_SAMPLE_RATE;
}

/**
 * One server message can carry several things at once (audio plus a caption
 * plus turnComplete), so this returns a list, in a sensible playback order.
 * Unknown fields are ignored so new API features do not break the app.
 */
export function parseServerMessage(raw: string): ServerEvent[] {
  let msg: Record<string, any>;
  try {
    msg = JSON.parse(raw);
  } catch {
    return [];
  }
  const events: ServerEvent[] = [];

  if (msg.setupComplete !== undefined) events.push({ type: 'setupComplete' });

  const sc = msg.serverContent;
  if (sc && typeof sc === 'object') {
    if (sc.interrupted) events.push({ type: 'interrupted' });
    const parts: any[] = sc.modelTurn?.parts ?? [];
    for (const p of parts) {
      const blob = p?.inlineData;
      if (blob?.data && String(blob.mimeType ?? '').startsWith('audio/')) {
        events.push({ type: 'audio', data: blob.data, sampleRate: rateFromMime(blob.mimeType) });
      }
    }
    if (typeof sc.inputTranscription?.text === 'string' && sc.inputTranscription.text) {
      events.push({ type: 'learnerText', text: sc.inputTranscription.text });
    }
    if (typeof sc.outputTranscription?.text === 'string' && sc.outputTranscription.text) {
      events.push({ type: 'tutorText', text: sc.outputTranscription.text });
    }
    if (sc.turnComplete) events.push({ type: 'turnComplete' });
  }

  if (msg.goAway) events.push({ type: 'goAway', timeLeftMs: parseDuration(msg.goAway.timeLeft) });

  const ru = msg.sessionResumptionUpdate;
  if (ru?.resumable && typeof ru.newHandle === 'string' && ru.newHandle) {
    events.push({ type: 'resumption', handle: ru.newHandle });
  }
  return events;
}
