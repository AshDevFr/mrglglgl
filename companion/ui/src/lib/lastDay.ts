// The day each character's journal was last open on, so coming back from another of its pages
// reopens it. Kept for the session only.
const days = new Map<number, string>();

export function rememberDay(characterId: number, day: string): void {
  days.set(characterId, day);
}

export function lastDay(characterId: number): string | undefined {
  return days.get(characterId);
}
