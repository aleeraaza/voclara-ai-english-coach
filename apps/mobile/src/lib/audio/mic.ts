/**
 * Microphone capture: streams 16 kHz mono chunks to a callback and, in
 * parallel, records the learner to a local M4A file (uploaded for
 * pronunciation scoring in a later step).
 *
 * Uses Android's `voiceCommunication` input, which turns on the platform's
 * echo cancellation and noise suppression, so hands-free mode works on the
 * phone speaker without the tutor hearing itself.
 */
import { AudioManager, AudioRecorder, FileFormat, FilePreset } from 'react-native-audio-api';

import { INPUT_SAMPLE_RATE } from '../live/gemini';
import { resample } from '../live/pcm';

/** 100 ms per chunk: low latency without flooding the socket. */
const CHUNK_SECONDS = 0.1;

export type MicPermission = 'granted' | 'denied';

export async function requestMicPermission(): Promise<MicPermission> {
  const status = await AudioManager.requestRecordingPermissions();
  return status === 'Granted' ? 'granted' : 'denied';
}

export class Mic {
  private recorder = new AudioRecorder({
    androidInputPreset: 'voiceCommunication',
    iosVoiceProcessing: true,
  });
  private running = false;

  /** `onChunk` receives Float32 samples already at 16 kHz. */
  async start(onChunk: (samples: Float32Array) => void): Promise<void> {
    if (this.running) return;
    try {
      await AudioManager.setAudioSessionActivity(true);
    } catch {
      // Android has no audio session to activate; iOS failure surfaces on start.
    }
    this.recorder.enableFileOutput({ format: FileFormat.M4A, preset: FilePreset.Low, channelCount: 1 });
    const ready = this.recorder.onAudioReady(
      {
        sampleRate: INPUT_SAMPLE_RATE,
        bufferLength: Math.round(INPUT_SAMPLE_RATE * CHUNK_SECONDS),
        channelCount: 1,
      },
      ({ buffer }) => {
        const raw = buffer.getChannelData(0);
        // The device may ignore the requested rate; Gemini needs exactly 16 kHz.
        onChunk(resample(raw, buffer.sampleRate, INPUT_SAMPLE_RATE));
      },
    );
    if (ready.status === 'error') throw new Error(ready.message);
    const started = await this.recorder.start();
    if (started.status === 'error') throw new Error(started.message);
    this.running = true;
  }

  /** Stop capture; returns the local recording path, if one was written. */
  async stop(): Promise<string | null> {
    if (!this.running) return null;
    this.running = false;
    this.recorder.clearOnAudioReady();
    const result = await this.recorder.stop();
    return result.status === 'success' ? (result.paths[0] ?? null) : null;
  }
}
