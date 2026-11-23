/** Human labels for settings values, shared by Profile and the settings screens. */
export const LANGUAGES = [
  { value: 'ur', label: 'Urdu', native: 'اردو' },
  { value: 'hi', label: 'Hindi', native: 'हिन्दी' },
  { value: 'bn', label: 'Bengali', native: 'বাংলা' },
  { value: 'ar', label: 'Arabic', native: 'العربية' },
  { value: 'en', label: 'English only', native: 'English' },
] as const;

export const LEVELS = [
  { value: 'a1', label: 'A1', hint: 'Beginner' },
  { value: 'a2', label: 'A2', hint: 'Elementary' },
  { value: 'b1', label: 'B1', hint: 'Intermediate' },
  { value: 'b2', label: 'B2', hint: 'Upper intermediate' },
  { value: 'c1', label: 'C1', hint: 'Advanced' },
  { value: 'c2', label: 'C2', hint: 'Proficient' },
] as const;

export const GOALS = [
  { value: 'ielts', label: 'IELTS', icon: 'school' },
  { value: 'interview', label: 'Job interviews', icon: 'briefcase' },
  { value: 'work', label: 'English at work', icon: 'business' },
  { value: 'abroad', label: 'Moving abroad', icon: 'airplane' },
  { value: 'daily', label: 'Daily conversation', icon: 'chatbubbles' },
] as const;

export function languageLabel(code?: string | null): string {
  return LANGUAGES.find((l) => l.value === code)?.label ?? 'Urdu';
}

export function levelLabel(code?: string | null): string {
  const l = LEVELS.find((x) => x.value === code);
  return l ? `${l.label} · ${l.hint}` : 'Not set';
}

export function goalLabel(code?: string | null): string {
  return GOALS.find((g) => g.value === code)?.label ?? 'Not set';
}
