import {
  ActionIcon,
  Alert,
  Button,
  Group,
  Menu,
  Select,
  Text,
  Textarea,
} from "@mantine/core";
import type { TFunction } from "i18next";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Job } from "@/bindings/Job";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { Story } from "@/bindings/Story";
import type { StoryKind } from "@/bindings/StoryKind";
import {
  iconOf,
  NarratorIcon,
  narratorOption,
  narratorSection,
} from "@/components/NarratorIcon";
import { WritingNow } from "@/components/WritingNow";
import { type Problem, problem } from "@/i18n/problems";
import { formatTime } from "@/lib/format";
import {
  defaultPlaceholder,
  narratorChoices,
  narratorLabel,
} from "@/lib/narrators";
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

/** The words a flagged entry mentions that the notes do not, quoted, from its stored reason. */
function flagged(t: TFunction, reason: string | null): string {
  return (reason ?? "")
    .split(", ")
    .filter(Boolean)
    .map((word) => t("writing.check.word", { word }))
    .join(", ");
}

/**
 * Says why the fact check flagged an entry, shown above its text: what the check does, what it
 * found, and that the find may be wrong. With `onKeep`, the player can vouch for the entry; with
 * `onWrite`, ask for another one.
 */
export function CheckNote({
  story,
  onKeep,
  onWrite,
  writeDisabled,
}: {
  story: Story;
  onKeep?: () => Promise<void>;
  onWrite?: () => void;
  writeDisabled?: boolean;
}) {
  const { t } = useTranslation();
  const [keeping, setKeeping] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <Alert
      color="yellow"
      className="check-note"
      title={t("writing.check.title")}
    >
      <Text size="sm">
        {t("writing.check.why", { words: flagged(t, story.checkReason) })}
      </Text>
      <Text size="sm" mt={4}>
        {t("writing.check.maybe")}
      </Text>
      <Text size="sm" mt={4}>
        {t(
          story.status === "draft"
            ? "writing.check.acceptIfRight"
            : "writing.check.untilKept",
        )}
      </Text>
      {(onKeep || onWrite) && (
        <Group gap="xs" mt="xs">
          {onKeep && (
            <Button
              size="xs"
              variant="light"
              color="yellow"
              loading={keeping}
              onClick={async () => {
                setKeeping(true);
                setError(null);
                try {
                  await onKeep();
                } catch (e) {
                  setError(String(e));
                } finally {
                  setKeeping(false);
                }
              }}
            >
              {t("writing.check.keep")}
            </Button>
          )}
          {onWrite && (
            <Button
              size="xs"
              variant="subtle"
              color="gray"
              disabled={writeDisabled}
              onClick={onWrite}
            >
              {t("writing.again")}
            </Button>
          )}
        </Group>
      )}
      {error && (
        <Text size="sm" c="red" mt={4}>
          {problem(t, "writing.keepFailed", { message: error })}
        </Text>
      )}
    </Alert>
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
  /** Extra actions for the entry shown, such as copying it, beside Edit. */
  actions?: (story: Story) => ReactNode;
  /** Rarer actions for the entry shown, as menu items under "More", after Write again. */
  menuItems?: (story: Story) => ReactNode;
  /** Saves the player's own text as a new version, made from the version shown. */
  onEdit?: (story: Story, text: string) => Promise<void>;
  /** Puts an entry the fact check flagged in the journal as it is. */
  onKeep?: (story: Story) => Promise<void>;
  /** A draft of this day or week waits in To review. */
  draftWaiting?: boolean;
  /** The narrator that writes when none is picked: the character's own, or Settings'. */
  defaultNarrator?: string;
  /** Opens this day or week's card in To review. */
  onOpenReview?: () => void;
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
  menuItems,
  onEdit,
  onKeep,
  draftWaiting,
  onOpenReview,
  defaultNarrator,
}: Props) {
  const { t, i18n } = useTranslation();
  const [index, setIndex] = useState(0);
  const [narrator, setNarrator] = useState<string | null>(null);
  // A new version arrived, or another day was opened: show the newest.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on these changes only
  useEffect(() => setIndex(0), [versions.length, day]);

  const shown = versions[index];
  const narratorName = (id: string) => narratorLabel(t, narrators, id);
  const [editing, setEditing] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  // Another day, or a new version: whatever was being edited belongs to the old one.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on these changes only
  useEffect(() => setEditing(null), [versions.length, day]);
  const editable = Boolean(onEdit && shown && !shown.partial && finished);
  // Writing again replaces what is on the page, so it waits behind More until asked for; a day
  // with nothing written yet offers it up front, as its main action.
  const [rewriting, setRewriting] = useState(false);
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on these changes only
  useEffect(() => setRewriting(false), [versions.length, day]);

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

  // A draft is written for To review: the entry on the page stays as it is meanwhile, and the
  // words appear on the To review card, where the draft will wait.
  const forReview = Boolean(job?.draft);
  const writingDraft = forReview && (live?.active || job?.state === "running");
  if (live?.active && !forReview) {
    return (
      <div className="entry-panel" aria-busy="true">
        <WritingNow
          label={t(loadingKey(day))}
          icon={iconOf(narrators, live.narrator)}
        />
        {live.text && <p className="entry streaming">{live.text}</p>}
        <Button variant="subtle" color="gray" onClick={onCancel}>
          {t("writing.cancel")}
        </Button>
      </div>
    );
  }

  const retrying = job?.state === "pending";
  const reviewButton = onOpenReview && (
    <Button size="xs" variant="light" w="fit-content" onClick={onOpenReview}>
      {t("writing.openReview")}
    </Button>
  );
  return (
    <div className="entry-panel" aria-busy={writingDraft || undefined}>
      {writingDraft && (
        <Alert color="teal" className="draft-note">
          <WritingNow
            label={t("writing.draftWriting")}
            icon={iconOf(narrators, live?.narrator)}
          />
          <Text size="sm" mt={4}>
            {t(shown ? "writing.draftWritingKeeps" : "writing.draftWritingNew")}
          </Text>
          <Group gap="xs" mt="xs">
            {reviewButton}
            <Button size="xs" variant="subtle" color="gray" onClick={onCancel}>
              {t("writing.cancel")}
            </Button>
          </Group>
        </Alert>
      )}
      {!writingDraft && draftWaiting && (
        <Alert color="teal" role="status" className="draft-note">
          <Text size="sm">
            {t(shown ? "writing.draftWaitsReplaces" : "writing.draftWaits")}
          </Text>
          {reviewButton && <Group mt="xs">{reviewButton}</Group>}
        </Alert>
      )}
      {live?.outcome === "cancelled" && (
        <Alert color="gray" role="status">
          {problem(t, "writing.outcome.cancelled")}
        </Alert>
      )}
      {/* A failure waits for its next try while attempts are left; after the last, the
          player decides. */}
      {live?.outcome === "timedOut" && (
        <Alert color="yellow" role="status">
          {problem(
            t,
            retrying
              ? "writing.outcome.timedOutRetrying"
              : "writing.outcome.timedOut",
          )}
        </Alert>
      )}
      {live?.outcome === "failed" && (
        <Alert color="red" role="status">
          {problem(
            t,
            retrying
              ? "writing.outcome.failedRetrying"
              : "writing.outcome.failed",
            { message: live.message ?? "" },
          )}
        </Alert>
      )}

      {editing !== null ? (
        <>
          <Textarea
            aria-label={t("writing.editLabel")}
            className="entry-editor"
            value={editing}
            onChange={(e) => setEditing(e.currentTarget.value)}
            autosize
            minRows={6}
          />
          <Text size="xs" c="dimmed">
            {t("writing.editHelp")} {t("writing.editMarginHelp")}
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
      ) : shown ? (
        <>
          {shown.check === "failed" && (
            <CheckNote
              story={shown}
              onKeep={onKeep && index === 0 ? () => onKeep(shown) : undefined}
              onWrite={
                finished && index === 0 ? () => onWrite(null) : undefined
              }
              writeDisabled={
                job?.state === "running" || job?.state === "pending"
              }
            />
          )}
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
          {/* Folded as under an entry: a full day's list runs long, and the photos, stamps
              and milestones beside it already say much of it. */}
          {facts && (
            <details className="facts">
              <summary>{t("writing.facts")}</summary>
              {facts}
            </details>
          )}
          {!shown && job?.state === "pending" && (
            <Text className="note">
              {t("writing.notYet", { position: job.position ?? 1 })}
              {blocked &&
                ` ${problem(t, `writing.blocked.${blocked}` as Problem)}`}
            </Text>
          )}
          {!shown && job?.state === "running" && !forReview && (
            <Text className="note">{t(loadingKey(day))}</Text>
          )}
          {!shown && !finished && (
            <Text className="note">{t(`writing.stillGoing.${kind}`)}</Text>
          )}
        </>
      )}

      {shown && job?.state === "pending" && (
        <Text className="note" role="status">
          {t(forReview ? "writing.againQueuedDraft" : "writing.againQueued", {
            position: job.position ?? 1,
          })}
          {blocked && ` ${problem(t, `writing.blocked.${blocked}` as Problem)}`}
        </Text>
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
        <Text size="xs" c="dimmed" className="byline">
          <NarratorIcon icon={iconOf(narrators, shown.narrator)} size={14} />
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

      {(finished || onWriteSoFar) && (!shown || rewriting) && (
        <Group gap="xs" mt="sm" wrap="wrap">
          <Select
            aria-label={t(`writing.narratorFor.${kind}`)}
            placeholder={defaultPlaceholder(
              t,
              narrators,
              defaultNarrator,
              t(`writing.narratorFor.${kind}`),
            )}
            data={narratorChoices(t, narrators, defaultNarrator)}
            renderOption={narratorOption(narrators)}
            leftSection={narratorSection(
              narrators,
              narrator ?? defaultNarrator,
            )}
            value={narrator}
            onChange={setNarrator}
            clearable
            size="xs"
            w={220}
          />
          <Button
            size="xs"
            variant={shown ? "light" : "filled"}
            disabled={job?.state === "running"}
            onClick={() => {
              setRewriting(false);
              if (finished) onWrite(narrator);
              else onWriteSoFar?.(narrator);
            }}
          >
            {finished
              ? shown
                ? t("writing.again")
                : t("writing.now")
              : shown
                ? t("writing.soFarAgain")
                : t("writing.soFar")}
          </Button>
          {rewriting && (
            <Button
              size="xs"
              variant="subtle"
              color="gray"
              onClick={() => setRewriting(false)}
            >
              {t("writing.editCancel")}
            </Button>
          )}
        </Group>
      )}
      {shown && editing === null && !rewriting && (
        <Group gap="xs" mt="sm" wrap="wrap">
          {actions?.(shown)}
          {editable && (
            <Button
              size="xs"
              variant="subtle"
              color="gray"
              onClick={() => setEditing(shown.text)}
            >
              {t("writing.edit")}
            </Button>
          )}
          {(finished || onWriteSoFar || menuItems) && (
            <Menu position="bottom-start" withinPortal>
              <Menu.Target>
                <ActionIcon
                  variant="subtle"
                  color="gray"
                  aria-label={t("writing.more")}
                >
                  ⋯
                </ActionIcon>
              </Menu.Target>
              <Menu.Dropdown>
                {(finished || onWriteSoFar) && (
                  <Menu.Item
                    disabled={job?.state === "running"}
                    onClick={() => setRewriting(true)}
                  >
                    {finished
                      ? t("writing.againMenu")
                      : t("writing.soFarAgainMenu")}
                  </Menu.Item>
                )}
                {menuItems?.(shown)}
              </Menu.Dropdown>
            </Menu>
          )}
        </Group>
      )}
    </div>
  );
}
