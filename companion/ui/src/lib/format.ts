/** A local calendar date ("2026-11-05") as a Date at local midnight. */
export function parseDay(day: string): Date {
  const [year, month, date] = day.split("-").map(Number);
  return new Date(year, month - 1, date);
}

/** A journal day ("2026-11-05") in the reader's language, read as a local date. */
export function formatDay(day: string, language: string, long = false): string {
  return new Intl.DateTimeFormat(language, {
    dateStyle: long ? "full" : "medium",
  }).format(parseDay(day));
}

/** "5 Nov", for stamps. */
export function formatDayShort(day: string, language: string): string {
  return new Intl.DateTimeFormat(language, {
    day: "numeric",
    month: "short",
  }).format(parseDay(day));
}

/** A Unix time as a date and time in the reader's language. */
export function formatTime(unix: number, language: string): string {
  return new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(unix * 1000));
}
