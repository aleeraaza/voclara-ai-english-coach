/**
 * Where an IELTS-style mock has got to, read from the examiner's captions.
 *
 * The examiner prompt (packages/prompts/examiner/v2.md) says fixed signal
 * sentences at each change: "Now, Part 2.", "You have one minute to
 * prepare.", "Now, Part 3." and "That is the end of the speaking test."
 * Captions arrive in fragments and the model may vary punctuation, so the
 * match is loose: case-insensitive, digits or words ("Part two").
 */
import type { Turn } from './live/transcript';

export type MockPart = 1 | 2 | 3;

export interface MockProgress {
  part: MockPart;
  /** The examiner has started the one-minute preparation (Part 2). */
  prepAnnounced: boolean;
  /** The examiner has closed the test. */
  ended: boolean;
}

/** Part 2 timings, in seconds. */
export const PREP_SECS = 60;
export const LONG_TURN_SECS = 120;

const PART2 = /\bpart\s*(2|two)\b/i;
const PART3 = /\bpart\s*(3|three)\b/i;
const PREP = /one\s+minute\s+to\s+prepare/i;
const END = /end\s+of\s+the\s+speaking\s+test/i;

export function mockProgress(turns: Turn[]): MockProgress {
  let part: MockPart = 1;
  let prepAnnounced = false;
  let ended = false;
  for (const turn of turns) {
    if (turn.role !== 'tutor') continue;
    const text = turn.text;
    // Parts only move forward: a mention of "Part 2" during Part 3 (e.g.
    // "linked to your Part 2 topic") must not send the learner back.
    if (PART3.test(text)) part = 3;
    else if (part === 1 && PART2.test(text)) part = 2;
    if (part === 2 && PREP.test(text)) prepAnnounced = true;
    if (END.test(text)) ended = true;
  }
  return { part, prepAnnounced, ended };
}

export type LongTurnStage =
  | { kind: 'waiting' }
  | { kind: 'prepare'; secsLeft: number }
  | { kind: 'speak'; secsLeft: number }
  | { kind: 'done' };

/** Part 2 clock: one minute to prepare, then up to two minutes to speak. */
export function longTurnStage(prepStartedAt: number | null, now: number): LongTurnStage {
  if (prepStartedAt === null) return { kind: 'waiting' };
  const elapsed = Math.max(0, Math.floor((now - prepStartedAt) / 1000));
  if (elapsed < PREP_SECS) return { kind: 'prepare', secsLeft: PREP_SECS - elapsed };
  const speaking = elapsed - PREP_SECS;
  if (speaking < LONG_TURN_SECS) return { kind: 'speak', secsLeft: LONG_TURN_SECS - speaking };
  return { kind: 'done' };
}
