/** Parses the battery-check interval field; returns null unless it is a whole number from 1 to 120. */
export function parsePollIntervalMinutes(input: string): number | null {
  const trimmed = input.trim();
  if (trimmed.length === 0) {
    return null;
  }
  const value = Number(trimmed);
  return Number.isInteger(value) && value >= 1 && value <= 120 ? value : null;
}
