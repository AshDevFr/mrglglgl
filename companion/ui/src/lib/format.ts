/** A journal day ("2026-11-05") in the reader's language, read as a local date. */
export function formatDay(day: string, language: string, long = false): string {
  const [year, month, date] = day.split("-").map(Number);
  return new Intl.DateTimeFormat(language, {
    dateStyle: long ? "full" : "medium",
  }).format(new Date(year, month - 1, date));
}

/** A Unix time as a date and time in the reader's language. */
export function formatTime(unix: number, language: string): string {
  return new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(unix * 1000));
}
