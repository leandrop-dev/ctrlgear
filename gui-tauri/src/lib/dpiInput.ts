/** Parses a comma-separated DPI preset list, as typed into the detail page's DPI field. */
export function parseDpiInput(input: string): { values: number[] } | { error: string } {
  const parts = input
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  if (parts.length === 0) {
    return { error: "At least one DPI value required" };
  }

  const values: number[] = [];
  for (const part of parts) {
    const value = Number(part);
    if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
      return { error: `Invalid DPI value: ${part}` };
    }
    values.push(value);
  }

  return { values };
}
