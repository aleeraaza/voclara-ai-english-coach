/**
 * Streamed playback of the tutor's voice: Gemini sends short 24 kHz PCM
 * chunks; each becomes an AudioBuffer appended to one queue source node.
 * When the learner talks over the tutor, `interrupt()` drops what is queued
 * so the tutor stops at once instead of finishing its sentence.
 */
import {
  AudioContext,
  type AudioBufferQueueSourceNode,
} from 'react-native-audio-api';

import { OUTPUT_SAMPLE_RATE } from '../live/gemini';

export class Player {
  private ctx = new AudioContext({
    sampleRate: OUTPUT_SAMPLE_RATE,
    // Same profile as the mic's voiceCommunication input, so Android's echo
    // canceller knows what is playing.
    androidOutputProfile: 'voiceCommunication',
  });
  private queue: AudioBufferQueueSourceNode | null = null;
  private speaking = false;
  private onSpeakingChange?: (speaking: boolean) => void;

  constructor(onSpeakingChange?: (speaking: boolean) => void) {
    this.onSpeakingChange = onSpeakingChange;
  }

  private ensureQueue(): AudioBufferQueueSourceNode {
    if (this.queue) return this.queue;
    const q = this.ctx.createBufferQueueSource();
    q.connect(this.ctx.destination);
    q.onbufferended = (e) => {
      if (e.isLastBufferInQueue) this.setSpeaking(false);
    };
    q.start(this.ctx.currentTime);
    this.queue = q;
    return q;
  }

  play(samples: Float32Array, sampleRate: number = OUTPUT_SAMPLE_RATE): void {
    if (samples.length === 0) return;
    const buffer = this.ctx.createBuffer(1, samples.length, sampleRate);
    buffer.copyToChannel(samples as Float32Array<ArrayBuffer>, 0);
    this.ensureQueue().enqueueBuffer(buffer);
    this.setSpeaking(true);
  }

  /** Stop the tutor mid-sentence (Gemini sent `interrupted`). */
  interrupt(): void {
    this.queue?.clearBuffers();
    this.setSpeaking(false);
  }

  async close(): Promise<void> {
    try {
      this.queue?.stop();
    } catch {
      // Already stopped.
    }
    this.queue = null;
    this.setSpeaking(false);
    await this.ctx.close();
  }

  private setSpeaking(v: boolean) {
    if (this.speaking === v) return;
    this.speaking = v;
    this.onSpeakingChange?.(v);
  }
}
