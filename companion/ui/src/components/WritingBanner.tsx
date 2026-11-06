import { Alert } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { WritingStatus } from "@/bindings/WritingStatus";
import { type Problem, problem } from "@/i18n/problems";

/**
 * What the writer is up to for this character, above the journal: why nothing is being written
 * and how many entries are waiting. A missing model or endpoint is shown even before any day is
 * waiting, so an empty journal never leaves the player guessing.
 */
export function WritingBanner({ status }: { status: WritingStatus }) {
  const { t } = useTranslation();
  const waiting = status.jobs.filter((j) => j.state !== "failed").length;
  const failed = status.jobs.filter((j) => j.state === "failed").length;
  // Waiting for the game to close or the charger only matters when something is waiting.
  const blocked =
    status.blocked &&
    (waiting > 0 || !["gameRunning", "onBattery"].includes(status.blocked))
      ? status.blocked
      : null;
  if (!blocked && waiting === 0 && failed === 0) return null;

  const lines = [
    waiting > 0 && t("writing.backlog.waiting", { count: waiting }),
    failed > 0 && t("writing.backlog.failed", { count: failed }),
    blocked && problem(t, `writing.blocked.${blocked}` as Problem),
  ].filter(Boolean);
  return (
    <Alert color={blocked ? "yellow" : "gray"} role="status">
      {lines.join(" ")}
    </Alert>
  );
}
