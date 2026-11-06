import { Alert, Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  cancelWriting,
  characterJournal,
  characterMilestones,
  characterScreenshots,
  characterStories,
  deleteCharacter,
  listCharacters,
  listNarrators,
  retryWriting,
  setWritingPaused,
  writeNow,
  writingStatus,
} from "@/api/commands";
import type { StoryKind } from "@/bindings/StoryKind";
import { CharacterHeader } from "@/components/CharacterHeader";
import { EmptyState } from "@/components/EmptyState";
import { type DayWriting, JournalPage } from "@/components/JournalPage";
import { CopyEntryButton, ExportDialog } from "@/components/Sharing";
import { WeekChapter } from "@/components/WeekChapter";
import { WeekNav } from "@/components/WeekNav";
import { type Problem, problem } from "@/i18n/problems";
import { daysNewestFirst, displayName } from "@/lib/journal";
import { useWriting } from "@/lib/useWriting";
import { dayKey, weekDays, weekStart } from "@/lib/weeks";
import { jobFor, slot, versionsOf } from "@/lib/writing";

interface Props {
  characterId: number;
  day?: string;
  /** Today, "YYYY-MM-DD"; injectable for tests. */
  today?: string;
}

export function CharacterPage({
  characterId,
  day,
  today = dayKey(new Date()),
}: Props) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const [exporting, setExporting] = useState(false);

  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const journal = useQuery({
    queryKey: ["journal", characterId],
    queryFn: () => characterJournal(characterId),
  });
  const awards = useQuery({
    queryKey: ["milestones", characterId],
    queryFn: () => characterMilestones(characterId),
  });
  const screenshots = useQuery({
    queryKey: ["screenshots", characterId],
    queryFn: () => characterScreenshots(characterId),
  });
  const stories = useQuery({
    queryKey: ["stories", characterId],
    queryFn: () => characterStories(characterId),
  });
  const writingState = useQuery({
    queryKey: ["writing", characterId],
    queryFn: () => writingStatus(characterId),
  });
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
  });
  const live = useWriting(characterId);
  const refreshStatus = () =>
    queryClient.invalidateQueries({ queryKey: ["writing", characterId] });

  /** What the entry panel needs for one day or week. */
  function writingFor(kind: StoryKind, start: string, end: string): DayWriting {
    return {
      finished: end < today,
      versions: versionsOf(stories.data ?? [], kind, start),
      live: live[slot(kind, start)],
      job: jobFor(writingState.data?.jobs ?? [], kind, start),
      blocked: writingState.data?.blocked ?? null,
      narrators: narrators.data ?? [],
      onWrite: (narrator) =>
        void writeNow(characterId, kind, start, narrator).then(refreshStatus),
      onCancel: () => void cancelWriting(characterId, kind, start),
      onRetry: () =>
        void retryWriting(characterId, kind, start).then(refreshStatus),
      actions: (story) => <CopyEntryButton story={story} />,
    };
  }

  const remove = useMutation({
    mutationFn: () => deleteCharacter(characterId),
    onSuccess: async () => {
      setConfirming(false);
      await queryClient.invalidateQueries();
      await navigate({ to: "/" });
    },
  });

  const summary = characters.data?.find((c) => c.character.id === characterId);
  if (journal.isError || characters.isError) {
    return (
      <div className="desk" style={{ padding: "2rem" }}>
        <Alert color="red">
          {problem(t, "errors.load", {
            message: String(journal.error ?? characters.error),
          })}
        </Alert>
      </div>
    );
  }
  if (journal.isPending || !summary) {
    return (
      <div className="desk" style={{ padding: "2rem" }}>
        <EmptyState title={t("loading")} />
      </div>
    );
  }
  const days = journal.data ? daysNewestFirst(journal.data) : [];
  const written = new Set(days);
  const selected = day ?? days[0] ?? today;
  const name = displayName(summary.character);
  const monday = weekStart(selected);
  const sunday = weekDays(selected)[6];
  const status = summary.character.status;

  function show(target: string) {
    navigate({
      to: "/characters/$characterId",
      params: { characterId: String(characterId) },
      search: { day: target },
    });
  }

  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <Stack gap="lg" maw="48rem">
        <Group justify="space-between" align="flex-start" wrap="nowrap">
          <CharacterHeader
            character={summary.character}
            journal={journal.data ?? null}
          />
          <Group gap="xs" wrap="nowrap">
            <Button
              variant="subtle"
              color="gray"
              onClick={() =>
                navigate({
                  to: "/characters/$characterId/milestones",
                  params: { characterId: String(characterId) },
                })
              }
            >
              {t("nav.milestones")}
            </Button>
            {writingState.data && (
              <Button
                variant="subtle"
                color="gray"
                onClick={() =>
                  void setWritingPaused(!writingState.data.paused).then(
                    refreshStatus,
                  )
                }
              >
                {writingState.data.paused
                  ? t("writing.resume")
                  : t("writing.pause")}
              </Button>
            )}
            <Button
              variant="subtle"
              color="gray"
              onClick={() => setExporting(true)}
            >
              {t("share.exportButton")}
            </Button>
            <Button
              variant="subtle"
              color="red"
              onClick={() => setConfirming(true)}
            >
              {t("character.remove")}
            </Button>
          </Group>
        </Group>
        {summary.archived && <Alert>{t("sync.archivedHelp")}</Alert>}
        {status && (
          <Alert color="yellow">
            {problem(t, `sync.status.${status}` as Problem)}
          </Alert>
        )}
        <WeekNav
          selected={selected}
          written={written}
          today={today}
          onSelect={show}
        />
        {journal.data && (
          <WeekChapter
            monday={monday}
            {...writingFor("week", monday, sunday)}
          />
        )}
        {journal.data && written.has(selected) ? (
          <JournalPage
            journal={journal.data}
            day={selected}
            awards={awards.data ?? []}
            screenshots={screenshots.data?.[selected] ?? []}
            writing={writingFor("day", selected, selected)}
          />
        ) : (
          <EmptyState title={t("week.empty")}>{t("week.emptyHelp")}</EmptyState>
        )}
      </Stack>
      <ExportDialog
        characterId={characterId}
        opened={exporting}
        onClose={() => setExporting(false)}
      />
      <Modal
        opened={confirming}
        onClose={() => setConfirming(false)}
        title={t("character.removeTitle", { name })}
      >
        <Stack>
          <Text size="sm">{t("character.removeBody")}</Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setConfirming(false)}>
              {t("character.cancel")}
            </Button>
            <Button
              color="red"
              loading={remove.isPending}
              onClick={() => remove.mutate()}
            >
              {t("character.removeConfirm")}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </div>
  );
}
