/**
 * Hand-off from the talk screen to the feedback screen. The transcript is too
 * large for route params, so it is kept here in memory.
 */
import type { Turn } from './live/transcript';
import type { EndReason } from './live/session';

export interface FinishedSession {
  sessionId: string;
  mode: string;
  reason: EndReason;
  secondsCharged: number | null;
  balanceSecs: number | null;
  transcript: Turn[];
  recordingPath: string | null;
  uploadError?: string;
  liveError?: string;
}

let last: FinishedSession | null = null;

export function setLastSession(s: FinishedSession): void {
  last = s;
}

export function getLastSession(): FinishedSession | null {
  return last;
}
