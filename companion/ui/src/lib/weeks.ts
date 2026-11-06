import { parseDay } from "./format";

/** A Date as a local calendar date key, "2026-11-05". */
export function dayKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function addDays(day: string, days: number): string {
  const date = parseDay(day);
  date.setDate(date.getDate() + days);
  return dayKey(date);
}

/** The Monday starting the week that contains `day`. */
export function weekStart(day: string): string {
  const weekday = (parseDay(day).getDay() + 6) % 7; // Monday is 0
  return addDays(day, -weekday);
}

/** The seven days, Monday to Sunday, of the week containing `day`. */
export function weekDays(day: string): string[] {
  const start = weekStart(day);
  return Array.from({ length: 7 }, (_, i) => addDays(start, i));
}

/**
 * The day to show after moving `weeks` weeks from `day`: the latest day with an entry in the
 * target week, or its Monday when nothing was written that week.
 */
export function shiftWeek(
  day: string,
  weeks: number,
  written: Set<string>,
): string {
  const target = weekDays(addDays(weekStart(day), weeks * 7));
  return [...target].reverse().find((d) => written.has(d)) ?? target[0];
}
