import { LONG_TURN_SECS, PREP_SECS, longTurnStage, mockProgress } from './mock';
import type { Turn } from './live/transcript';

const tutor = (text: string, t_ms = 0): Turn => ({ role: 'tutor', text, t_ms });
const learner = (text: string, t_ms = 0): Turn => ({ role: 'learner', text, t_ms });

describe('mockProgress', () => {
  it('starts in Part 1', () => {
    expect(mockProgress([])).toEqual({ part: 1, prepAnnounced: false, ended: false });
    expect(mockProgress([tutor("Hello. Let's begin Part 1. Where do you live?")]).part).toBe(1);
  });

  it('follows the examiner through the parts', () => {
    const turns = [
      tutor("Let's begin Part 1."),
      learner('I live in Lahore.'),
      tutor('Thank you. Now, Part 2. Describe a memorable journey.'),
    ];
    expect(mockProgress(turns)).toMatchObject({ part: 2, prepAnnounced: false });
    turns.push(tutor('You have one minute to prepare. Start speaking when you are ready.'));
    expect(mockProgress(turns)).toMatchObject({ part: 2, prepAnnounced: true });
    turns.push(tutor('Now, Part 3. Why do people enjoy travelling?'));
    expect(mockProgress(turns)).toMatchObject({ part: 3, ended: false });
    turns.push(tutor('That is the end of the speaking test. Your feedback is on its way.'));
    expect(mockProgress(turns)).toMatchObject({ part: 3, ended: true });
  });

  it('accepts words, odd spacing and lower case', () => {
    expect(mockProgress([tutor('now, part two.')]).part).toBe(2);
    expect(mockProgress([tutor('Now part   3, please.')]).part).toBe(3);
  });

  it('never moves backwards and ignores the learner', () => {
    const turns = [tutor('Now, Part 3.'), tutor('This links to your Part 2 topic.')];
    expect(mockProgress(turns).part).toBe(3);
    expect(mockProgress([learner('Can we do part 2 now?')]).part).toBe(1);
  });

  it('only counts the prep signal in Part 2', () => {
    expect(mockProgress([tutor('You have one minute to prepare.')]).prepAnnounced).toBe(false);
  });

  it('does not mistake larger numbers for a part', () => {
    expect(mockProgress([tutor('Part 23 of the book')]).part).toBe(1);
  });
});

describe('longTurnStage', () => {
  const t0 = 1_000_000;
  it('waits until preparation starts', () => {
    expect(longTurnStage(null, t0)).toEqual({ kind: 'waiting' });
  });
  it('counts one minute to prepare, then two to speak', () => {
    expect(longTurnStage(t0, t0)).toEqual({ kind: 'prepare', secsLeft: PREP_SECS });
    expect(longTurnStage(t0, t0 + 59_500)).toEqual({ kind: 'prepare', secsLeft: 1 });
    expect(longTurnStage(t0, t0 + 60_000)).toEqual({ kind: 'speak', secsLeft: LONG_TURN_SECS });
    expect(longTurnStage(t0, t0 + 179_000)).toEqual({ kind: 'speak', secsLeft: 1 });
    expect(longTurnStage(t0, t0 + 180_000)).toEqual({ kind: 'done' });
  });
});
