import type { TFunction } from "i18next";

/**
 * Every problem the app can show, reviewed: an error, or a reason the journal is waiting. Each
 * key has a sibling `<key>Next` in the catalogue that says, plainly, what to do next, and the
 * window shows both together through `problem()`. A test over the catalogue fails when a key
 * here lacks its next step, when the catalogue gains an error-like key that is not listed, or
 * when a component shows one of these messages without its next step.
 */
export const PROBLEMS = [
  "errors.load",
  "settings.foundNone",
  "settings.chosenMissing",
  "settings.error",
  "settings.connectionFailed",
  "models.failed",
  "models.notEnoughMemory",
  "models.notEnoughDisk",
  "share.copyFailed",
  "share.exportFailed",
  "sync.status.older",
  "sync.status.newer",
  "sync.status.unreadable",
  "writing.blocked.paused",
  "writing.blocked.noModel",
  "writing.blocked.noEndpoint",
  "writing.blocked.gameRunning",
  "writing.blocked.onBattery",
  "writing.outcome.cancelled",
  "writing.outcome.timedOut",
  "writing.outcome.failed",
  "writing.reason.cancelled",
  "writing.reason.timedOut",
  "writing.reason.other",
  "writing.setAside",
  "narrators.previewSetAside",
  "narrators.previewFailed",
  "narrators.fileFailed",
  "site.buildFailed",
  "update.failed",
  "log.copyFailed",
  "addon.problem.newerSchema",
  "addon.problem.updatePending",
  "addon.problem.outOfDate",
  "addon.problem.errors",
  "addon.actionFailed",
  "addon.releaseFailed",
  "addon.staleListing",
  "log.showFailed",
  "settings.unreadable",
  "writing.editFailed",
  "failure.offline",
  "failure.serverTrouble",
  "failure.badPackage",
  "failure.gameRunning",
  "failure.noSpace",
  "failure.appTooOld",
] as const;

export type Problem = (typeof PROBLEMS)[number];

/** The message and what to do about it, as one text. */
export function problem(
  t: TFunction,
  key: Problem,
  values: Record<string, unknown> = {},
): string {
  const message = t(key, values);
  // Messages ending in a detail from the backend get their full stop here.
  const stop = /[.!?]$/.test(message) ? "" : ".";
  return `${message}${stop} ${t(`${key}Next`, values)}`;
}
