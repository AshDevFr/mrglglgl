import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

/** Whether lines reach the app's log file. Off in the unit tests, where every command is a mock
 * and a log line would be one more call to account for, and in the documentation's demo, which
 * has no app behind it. */
function enabled(): boolean {
  return import.meta.env.MODE !== "test" && import.meta.env.VITE_DEMO !== "1";
}

/** Writes a line to the app's log file, under `mrglglgl::window`, and to the console. Never
 * throws: a log that cannot be written must not break what it describes. */
export function log(level: LogLevel, message: string): void {
  const line = `[window] ${message}`;
  if (level === "error") console.error(line);
  else if (level === "warn") console.warn(line);
  else if (level === "info") console.info(line);
  else console.debug(line);
  if (!enabled()) return;
  tauriInvoke("log_from_window", { level, message }).catch(() => {});
}

/** An error's message, with its stack when it has one. */
export function describe(error: unknown): string {
  if (error instanceof Error) return error.stack ?? error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

/** Calls a command and logs it when it fails, with how long it took, so every error the window
 * shows is also in the log file. The error is passed on unchanged. */
export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const started = performance.now();
  try {
    return await tauriInvoke<T>(command, args);
  } catch (error) {
    const took = Math.round(performance.now() - started);
    log(
      "warn",
      `command ${command} failed after ${took}ms: ${describe(error)}`,
    );
    throw error;
  }
}

/** Sends what breaks outside React to the log: an uncaught error or a rejected promise nobody
 * handled. */
export function logUncaught(): void {
  window.addEventListener("error", (event) => {
    log(
      "error",
      `uncaught error: ${describe(event.error ?? event.message)} at ${event.filename}:${event.lineno}`,
    );
  });
  window.addEventListener("unhandledrejection", (event) => {
    log("error", `unhandled rejection: ${describe(event.reason)}`);
  });
}
