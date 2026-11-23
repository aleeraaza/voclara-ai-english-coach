/** "9:05" from seconds. */
export function formatClock(totalSecs: number): string {
  const s = Math.max(0, Math.round(totalSecs));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** "12 min" / "45 s" for a minute balance. */
export function formatMinutes(secs: number): string {
  if (secs >= 60) return `${Math.floor(secs / 60)} min`;
  return `${Math.max(0, Math.round(secs))} s`;
}

/** "Good morning" etc. for the home greeting. */
export function greeting(date = new Date()): string {
  const h = date.getHours();
  if (h < 5) return 'Up late';
  if (h < 12) return 'Good morning';
  if (h < 17) return 'Good afternoon';
  return 'Good evening';
}
