/** Parses the sleep-timer minutes field; returns null unless it is a whole number from 0 to 255. */
export function parseSleepTimerMinutes(input: string): number | null {
  const trimmed = input.trim();
  if (trimmed.length === 0) {
    return null;
  }
  const value = Number(trimmed);
  return Number.isInteger(value) && value >= 0 && value <= 255 ? value : null;
}
