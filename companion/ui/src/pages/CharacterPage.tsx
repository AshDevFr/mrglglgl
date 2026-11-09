import { Alert, Stack } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import {
  cancelWriting,
  characterJournal,
  characterMilestones,
  characterScreenshots,
  characterStories,
  editStory,
  getSettings,
  listCharacters,
  listNarrators,
  retryWriting,
  reviewList,
  writeNow,
  writeSoFar,
  writingStatus,
} from "@/api/commands";
import type { StoryKind } from "@/bindings/StoryKind";
import { CharacterTop } from "@/components/CharacterTop";
import { EmptyState } from "@/components/EmptyState";
import { type DayWriting, JournalPage } from "@/components/JournalPage";
import { CopyEntryButton, DebugExportButton } from "@/components/Sharing";
import { WeekChapter } from "@/components/WeekChapter";
import { WeekNav } from "@/components/WeekNav";
import { WritingBanner } from "@/components/WritingBanner";
import { type Problem, problem } from "@/i18n/problems";
import { daysNewestFirst } from "@/lib/journal";
import { rememberDay } from "@/lib/lastDay";
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
  const review = useQuery({
    queryKey: ["review", characterId],
    queryFn: () => reviewList(characterId),
  });
  const settings = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  // Weekly chapters off: a week shows its chapter only if one was written before.
  const weekly = settings.data?.settings.writing.weekly ?? true;
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
      onWriteSoFar:
        kind === "day" && start === today
          ? (narrator) =>
              void writeSoFar(characterId, narrator).then(refreshStatus)
          : undefined,
      onCancel: () => void cancelWriting(characterId, kind, start),
      onRetry: () =>
        void retryWriting(characterId, kind, start).then(refreshStatus),
      actions: (story) => (
        <>
          <CopyEntryButton story={story} />
          <DebugExportButton story={story} />
        </>
      ),
      onEdit: async (story, text) => {
        const saved = await editStory(characterId, story.id, text);
        if (saved === null) throw new Error(t("writing.editRefused"));
        await queryClient.invalidateQueries();
      },
    };
  }

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
  const monday = weekStart(selected);
  const sunday = weekDays(selected)[6];
  const status = summary.character.status;

  function show(target: string) {
    rememberDay(characterId, target);
    navigate({
      to: "/characters/$characterId",
      params: { characterId: String(characterId) },
      search: { day: target },
    });
  }

  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <Stack gap="lg">
        <CharacterTop characterId={characterId} current="journal" />
        {summary.archived && <Alert>{t("sync.archivedHelp")}</Alert>}
        {status && (
          <Alert color="yellow">
            {problem(t, `sync.status.${status}` as Problem)}
          </Alert>
        )}
        {writingState.data && (
          <WritingBanner status={writingState.data} drafts={summary.toReview} />
        )}
        <WeekNav
          selected={selected}
          written={written}
          today={today}
          onSelect={show}
        />
        {journal.data &&
          (weekly ||
            versionsOf(stories.data ?? [], "week", monday).length > 0) && (
            <WeekChapter
              monday={monday}
              {...writingFor("week", monday, sunday)}
            />
          )}
        {review.data?.some(
          (i) =>
            i.kind === "day" && i.startDay === selected && i.drafts.length > 0,
        ) && <Alert color="teal">{t("review.draftWaitingHere")}</Alert>}
        {journal.data && written.has(selected) ? (
          <JournalPage
            journal={journal.data}
            day={selected}
            awards={awards.data ?? []}
            screenshots={screenshots.data?.[selected] ?? []}
            writing={writingFor("day", selected, selected)}
          />
        ) : (
          <EmptyState title={t("week.empty")} art="sleep">
            {t("week.emptyHelp")}
          </EmptyState>
        )}
      </Stack>
    </div>
  );
}
