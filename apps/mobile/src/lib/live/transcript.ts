/**
 * Builds the session transcript from streamed caption fragments.
 *
 * Gemini sends captions in small pieces ("I go", " to office", ...). Pieces
 * from the same speaker join into one turn; a change of speaker (or the
 * tutor finishing its turn) starts a new one. The result is what the API
 * stores and the grader reads: `[{role, text, t_ms}]`.
 */

export type Role = 'learner' | 'tutor';

export interface Turn {
  role: Role;
  text: string;
  /** Milliseconds from session start to the first fragment of this turn. */
  t_ms: number;
}

export class TranscriptBuilder {
  private turns: Turn[] = [];
  private closed = true;

  constructor(private startedAt: number, private now: () => number = Date.now) {}

  add(role: Role, fragment: string): void {
    if (!fragment) return;
    const last = this.turns[this.turns.length - 1];
    if (last && last.role === role && !this.closed) {
      last.text = joinFragments(last.text, fragment);
      return;
    }
    this.turns.push({
      role,
      text: fragment.trimStart(),
      t_ms: Math.max(0, this.now() - this.startedAt),
    });
    this.closed = false;
  }

  /** End of the tutor's turn: the next fragment starts a new turn. */
  closeTurn(): void {
    this.closed = true;
  }

  /** Clean copy, without empty turns, for upload and display. */
  toJSON(): Turn[] {
    return this.turns
      .map((t) => ({ ...t, text: t.text.replace(/\s+/g, ' ').trim() }))
      .filter((t) => t.text.length > 0);
  }

  get length(): number {
    return this.turns.length;
  }
}

/** Join caption fragments, adding a space only where one is missing. */
export function joinFragments(a: string, b: string): string {
  if (!a) return b;
  if (/\s$/.test(a) || /^[\s.,!?;:'’)]/.test(b)) return a + b;
  return `${a} ${b}`;
}
