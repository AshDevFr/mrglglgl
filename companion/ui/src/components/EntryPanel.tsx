import { Alert, Button, Group, Select, Text, Textarea } from "@mantine/core";
import type { TFunction } from "i18next";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { Story } from "@/bindings/Story";
import type { StoryKind } from "@/bindings/StoryKind";
import { type Problem, problem } from "@/i18n/problems";
import { formatTime } from "@/lib/format";
import { narratorLabel } from "@/lib/narrators";
import { type Live, loadingKey } from "@/lib/writing";

/** An entry's text: paragraphs, with the murloc's margin note set apart. */
export function EntryText({ text }: { text: string }) {
  const paragraphs = text
    .split(/\n\s*\n/)
    .map((p) => p.trim())
    .filter(Boolean);
  return (
    <>
      {paragraphs.map((p) =>
        p.startsWith("*") ? (
          <p key={p} className="margin-note">
            {p.replace(/^\*\s*/, "")}
          </p>
        ) : (
          <p key={p} className="entry">
            {p}
          </p>
        ),
      )}
    </>
  );
}

interface Props {
  kind: StoryKind;
  day: string;
  /** Days and weeks still going are not written yet. */
  finished: boolean;
  /** Every version, newest first. */
  versions: Story[];
  live?: Live;
  job?: Job;
  /** Why the writer is waiting, from the writing status. */
  blocked: string | null;
  narrators: NarratorInfo[];
  /** The plain facts: shown when there is no entry to read, folded away when there is. */
  facts?: ReactNode;
  onWrite: (narrator: string | null) => void;
  /** Tells a day that is still going so far; offered only on such a day. */
  onWriteSoFar?: (narrator: string | null) => void;
  onCancel: () => void;
  onRetry: () => void;
  /** Extra actions for the entry shown, such as copying it. */
  actions?: (story: Story) => ReactNode;
  /** Saves the player's own text as a new version, made from the version shown. */
  onEdit?: (story: Story, text: string) => Promise<void>;
}

function failure(t: TFunction, reason: string | null) {
  if (reason === "cancelled") return problem(t, "writing.reason.cancelled");
  if (reason === "timedOut") return problem(t, "writing.reason.timedOut");
  return problem(t, "writing.reason.other", { message: reason ?? "" });
}

export function EntryPanel({
  kind,
  day,
  finished,
  versions,
  live,
  job,
  blocked,
  narrators,
  facts,
  onWrite,
  onWriteSoFar,
  onCancel,
  onRetry,
  actions,
  onEdit,
}: Props) {
  const { t, i18n } = useTranslation();
  const [index, setIndex] = useState(0);
  const [narrator, setNarrator] = useState<string | null>(null);
  // A new version arrived, or another day was opened: show the newest.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on these changes only
  useEffect(() => setIndex(0), [versions.length, day]);

  const shown = versions[index];
  const readable = shown && shown.check !== "failed";
  const narratorName = (id: string) => narratorLabel(t, narrators, id);
  const [editing, setEditing] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  // Another day, or a new version: whatever was being edited belongs to the old one.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on these changes only
  useEffect(() => setEditing(null), [versions.length, day]);
  const editable = Boolean(onEdit && shown && !shown.partial && finished);

  async function saveEdit() {
    if (!onEdit || !shown || editing === null) return;
    setSaving(true);
    setEditError(null);
    try {
      await onEdit(shown, editing);
      setEditing(null);
    } catch (e) {
      setEditError(String(e));
    } finally {
      setSaving(false);
    }
  }

  if (live?.active) {
    return (
      <div className="entry-panel" aria-busy="true">
        <Text className="writing-status" role="status">
          {t(loadingKey(day))}
        </Text>
        {live.text && (
          <p className="entry streaming" aria-live="polite">
            {live.text}
          </p>
        )}
        <Button variant="subtle" color="gray" onClick={onCancel}>
          {t("writing.cancel")}
        </Button>
      </div>
    );
  }

  return (
    <div className="entry-panel">
      {live?.outcome === "cancelled" && (
        <Alert color="gray" role="status">
          {problem(t, "writing.outcome.cancelled")}
        </Alert>
      )}
      {live?.outcome === "timedOut" && (
        <Alert color="yellow" role="status">
          {problem(t, "writing.outcome.timedOut")}
        </Alert>
      )}
      {live?.outcome === "failed" && (
        <Alert color="red" role="status">
          {problem(t, "writing.outcome.failed", {
            message: live.message ?? "",
          })}
        </Alert>
      )}

      {editing !== null ? (
        <>
          <Textarea
            aria-label={t("writing.editLabel")}
            value={editing}
            onChange={(e) => setEditing(e.currentTarget.value)}
            autosize
            minRows={6}
          />
          <Text size="xs" c="dimmed">
            {t("writing.editHelp")}
          </Text>
          {editError && (
            <Alert color="red">
              {problem(t, "writing.editFailed", { message: editError })}
            </Alert>
          )}
          <Group gap="xs">
            <Button
              size="xs"
              loading={saving}
              disabled={editing.trim() === ""}
              onClick={() => void saveEdit()}
            >
              {t("writing.editSave")}
            </Button>
            <Button
              size="xs"
              variant="default"
              disabled={saving}
              onClick={() => setEditing(null)}
            >
              {t("writing.editCancel")}
            </Button>
          </Group>
        </>
      ) : readable ? (
        <>
          <EntryText text={shown.text} />
          {shown.partial && !finished && (
            <Text className="note">{t("writing.soFarNote")}</Text>
          )}
          {facts && (
            <details className="facts">
              <summary>{t("writing.facts")}</summary>
              {facts}
            </details>
          )}
        </>
      ) : (
        <>
          {shown?.check === "failed" && (
            <Text className="note">
              {problem(t, "writing.setAside", {
                reason: shown.checkReason ?? "",
              })}
            </Text>
          )}
          {facts}
          {!shown && job?.state === "pending" && (
            <Text className="note">
              {t("writing.notYet", { position: job.position ?? 1 })}
              {blocked &&
                ` ${problem(t, `writing.blocked.${blocked}` as Problem)}`}
            </Text>
          )}
          {!shown && job?.state === "running" && (
            <Text className="note">{t(loadingKey(day))}</Text>
          )}
          {!shown && !finished && (
            <Text className="note">{t(`writing.stillGoing.${kind}`)}</Text>
          )}
        </>
      )}

      {job?.state === "failed" && (
        <Group gap="xs" className="note">
          <Text size="sm">{failure(t, job.lastError)}</Text>
          <Button size="xs" variant="light" onClick={onRetry}>
            {t("writing.retry")}
          </Button>
        </Group>
      )}

      {shown && versions.length > 1 && (
        <Group gap="xs" className="versions">
          <Button
            size="xs"
            variant="subtle"
            color="gray"
            disabled={index >= versions.length - 1}
            onClick={() => setIndex(index + 1)}
          >
            {t("writing.older")}
          </Button>
          <Text size="sm">
            {t("writing.version", {
              n: versions.length - index,
              total: versions.length,
            })}
          </Text>
          <Button
            size="xs"
            variant="subtle"
            color="gray"
            disabled={index === 0}
            onClick={() => setIndex(index - 1)}
          >
            {t("writing.newer")}
          </Button>
        </Group>
      )}
      {shown && (
        <Text size="xs" c="dimmed">
          {t(
            shown.check === "edited"
              ? "writing.editedByline"
              : shown.partial
                ? "writing.soFarByline"
                : "writing.byline",
            {
              narrator: narratorName(shown.narrator),
              when: formatTime(shown.createdAt, i18n.language),
            },
          )}
        </Text>
      )}

      {(finished || onWriteSoFar) && (
        <Group gap="xs" mt="sm" wrap="wrap">
          <Select
            aria-label={t(`writing.narratorFor.${kind}`)}
            placeholder={t(`writing.narratorFor.${kind}`)}
            data={narrators.map((n) => ({ value: n.id, label: n.name }))}
            value={narrator}
            onChange={setNarrator}
            clearable
            size="xs"
            w={220}
          />
          <Button
            size="xs"
            variant="light"
            disabled={job?.state === "running"}
            onClick={() =>
              finished ? onWrite(narrator) : onWriteSoFar?.(narrator)
            }
          >
            {finished
              ? shown
                ? t("writing.again")
                : t("writing.now")
              : shown
                ? t("writing.soFarAgain")
                : t("writing.soFar")}
          </Button>
          {readable && actions?.(shown)}
          {editable && editing === null && (
            <Button
              size="xs"
              variant="subtle"
              onClick={() => setEditing(shown.text)}
            >
              {t("writing.edit")}
            </Button>
          )}
        </Group>
      )}
    </div>
  );
}
