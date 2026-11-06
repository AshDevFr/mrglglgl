import {
  Button,
  Checkbox,
  Group,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  acceptDraft,
  characterJournal,
  discardDraft,
  listNarrators,
  reviewList,
  writeDrafts,
} from "@/api/commands";
import type { Journal } from "@/bindings/Journal";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { ReviewItem } from "@/bindings/ReviewItem";
import { type Problem, problem } from "@/i18n/problems";
import { formatDay } from "@/lib/format";
import { dayFacts } from "@/lib/journal";
import { narratorLabel } from "@/lib/narrators";
import { daySentences } from "@/lib/sentences";
import { scheduleRebuild } from "@/lib/siteBuild";
import { useWriting } from "@/lib/useWriting";
import { type Live, slot } from "@/lib/writing";
import { EmptyState } from "./EmptyState";
import { EntryText } from "./EntryPanel";

/** The longest rewrite note the writer takes; the backend cuts anything longer. */
const NOTE_CHARS = 120;

type Key = string;
const keyOf = (item: ReviewItem): Key => slot(item.kind, item.startDay);

interface Actions {
  /** Writes these items as drafts, with an optional narrator and note. */
  write: (
    items: ReviewItem[],
    narrator: string | null,
    note: string | null,
  ) => void;
  accept: (storyId: number) => void;
  discard: (storyId: number) => void;
}

/** A character's finished days and weeks waiting for an entry, and the drafts waiting for the
 * player: write them, accept them, rewrite them with a note, or throw them away. */
export function ReviewList({ characterId }: { characterId: number }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<Set<Key>>(new Set());
  const [narrator, setNarrator] = useState<string | null>(null);
  const live = useWriting(characterId);

  const items = useQuery({
    queryKey: ["review", characterId],
    queryFn: () => reviewList(characterId),
  });
  const journal = useQuery({
    queryKey: ["journal", characterId],
    queryFn: () => characterJournal(characterId),
  });
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
  });

  const refresh = () =>
    Promise.all(
      [
        ["review", characterId],
        ["stories", characterId],
        ["characters"],
        ["writing"],
      ].map((queryKey) => queryClient.invalidateQueries({ queryKey })),
    );
  const actions: Actions = {
    write: (chosen, who, note) =>
      void writeDrafts(
        characterId,
        chosen.map((i) => ({ kind: i.kind, startDay: i.startDay })),
        who,
        note,
      ).then(refresh),
    accept: (id) =>
      void acceptDraft(characterId, id).then(() => {
        // An accepted entry belongs on the website too.
        scheduleRebuild();
        return refresh();
      }),
    discard: (id) => void discardDraft(characterId, id).then(refresh),
  };

  if (items.isPending) return <EmptyState title={t("loading")} />;
  const list = items.data ?? [];
  if (list.length === 0) return <EmptyState title={t("review.empty")} />;

  const chosen = list.filter((i) => selected.has(keyOf(i)));
  const toggle = (key: Key) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const each = (f: (i: ReviewItem) => void) => () => {
    chosen.forEach(f);
    setSelected(new Set());
  };

  return (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        {t("review.intro")}
      </Text>
      <Group gap="xs" wrap="wrap">
        <Select
          aria-label={t("review.narrator")}
          placeholder={t("review.usualNarrator")}
          data={(narrators.data ?? []).map((n) => ({
            value: n.id,
            label: n.name,
          }))}
          value={narrator}
          onChange={setNarrator}
          clearable
          size="xs"
          w={220}
        />
        <Button
          size="xs"
          variant="light"
          disabled={chosen.length === 0}
          onClick={() => {
            actions.write(chosen, narrator, null);
            setSelected(new Set());
          }}
        >
          {t("review.writeSelected", { count: chosen.length })}
        </Button>
        <Button
          size="xs"
          variant="subtle"
          disabled={!chosen.some((i) => i.drafts.length > 0)}
          onClick={each((i) => i.drafts[0] && actions.accept(i.drafts[0].id))}
        >
          {t("review.acceptSelected")}
        </Button>
        <Button
          size="xs"
          variant="subtle"
          color="red"
          disabled={!chosen.some((i) => i.drafts.length > 0)}
          onClick={each((i) => {
            for (const d of i.drafts) actions.discard(d.id);
          })}
        >
          {t("review.discardSelected")}
        </Button>
      </Group>
      {list.map((item) => (
        <ReviewRow
          key={keyOf(item)}
          item={item}
          journal={journal.data ?? null}
          narrators={narrators.data ?? []}
          narrator={narrator}
          live={live[keyOf(item)]}
          selected={selected.has(keyOf(item))}
          onToggle={() => toggle(keyOf(item))}
          actions={actions}
        />
      ))}
    </Stack>
  );
}

interface RowProps {
  item: ReviewItem;
  journal: Journal | null;
  narrators: NarratorInfo[];
  /** The narrator chosen above the list, for a plain write. */
  narrator: string | null;
  live?: Live;
  selected: boolean;
  onToggle: () => void;
  actions: Actions;
}

function ReviewRow({
  item,
  journal,
  narrators,
  narrator,
  live,
  selected,
  onToggle,
  actions,
}: RowProps) {
  const { t, i18n } = useTranslation();
  const [rewriting, setRewriting] = useState(false);
  const [rewriteNarrator, setRewriteNarrator] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const draft = item.drafts[0];
  const title =
    item.kind === "week"
      ? t("writing.chapter", {
          date: formatDay(item.startDay, i18n.language, true),
        })
      : formatDay(item.startDay, i18n.language, true);
  const narratorName = (id: string) => narratorLabel(t, narrators, id);
  const day = item.kind === "day" ? journal?.days[item.startDay] : undefined;
  const facts =
    journal && day
      ? daySentences(dayFacts(journal, day), t, i18n.language).join(" ")
      : null;
  const busy = live?.active || item.job?.state === "running";

  return (
    <article className="journal-page" aria-label={title}>
      <Group gap="sm" wrap="nowrap" align="flex-start">
        <Checkbox
          aria-label={t("review.select", { day: title })}
          checked={selected}
          onChange={onToggle}
          mt={6}
        />
        <Stack gap="xs" style={{ flex: 1 }}>
          <h3>{title}</h3>
          {live?.active ? (
            <p className="entry streaming" aria-live="polite">
              {live.text}
            </p>
          ) : draft ? (
            <>
              <EntryText text={draft.text} />
              <Text size="xs" c="dimmed">
                {t("review.draftBy", {
                  narrator: narratorName(draft.narrator),
                })}
                {draft.note && ` · ${t("review.asked", { note: draft.note })}`}
              </Text>
              {item.hasEntry && (
                <Text size="sm" className="note">
                  {t("review.replaces")}
                </Text>
              )}
            </>
          ) : (
            <Text className="note">
              {item.discarded ? t("review.discarded") : t("review.notWritten")}
            </Text>
          )}
          {!live?.active && item.job?.state === "pending" && (
            <Text size="sm" className="note">
              {t("review.waiting", { position: item.job.position ?? 1 })}
            </Text>
          )}
          {!live?.active && item.job?.state === "running" && (
            <Text size="sm" className="note">
              {t("review.running")}
            </Text>
          )}
          {item.job?.state === "failed" && (
            <Text size="sm" className="note">
              {problem(
                t,
                (item.job.lastError === "cancelled"
                  ? "writing.reason.cancelled"
                  : item.job.lastError === "timedOut"
                    ? "writing.reason.timedOut"
                    : "writing.reason.other") as Problem,
                { message: item.job.lastError ?? "" },
              )}
            </Text>
          )}
          {facts && (
            <details className="facts">
              <summary>{t("review.facts")}</summary>
              <p className="entry">{facts}</p>
            </details>
          )}

          {rewriting ? (
            <Stack gap="xs">
              <Select
                aria-label={t("writing.narratorFor.day")}
                placeholder={t("review.usualNarrator")}
                data={narrators.map((n) => ({ value: n.id, label: n.name }))}
                value={rewriteNarrator}
                onChange={setRewriteNarrator}
                clearable
                size="xs"
                w={220}
              />
              <TextInput
                label={t("review.note")}
                description={t("review.noteHelp")}
                placeholder={t("review.notePlaceholder")}
                value={note}
                maxLength={NOTE_CHARS}
                onChange={(e) => setNote(e.currentTarget.value)}
                size="xs"
              />
              <Group gap="xs">
                <Button
                  size="xs"
                  variant="light"
                  onClick={() => {
                    actions.write([item], rewriteNarrator, note.trim() || null);
                    setRewriting(false);
                    setNote("");
                  }}
                >
                  {t("review.writeDraft")}
                </Button>
                <Button
                  size="xs"
                  variant="subtle"
                  color="gray"
                  onClick={() => setRewriting(false)}
                >
                  {t("review.cancel")}
                </Button>
              </Group>
            </Stack>
          ) : (
            <Group gap="xs" wrap="wrap">
              {draft && (
                <Button size="xs" onClick={() => actions.accept(draft.id)}>
                  {t("review.accept")}
                </Button>
              )}
              {draft || item.discarded || item.hasEntry ? (
                <Button
                  size="xs"
                  variant="light"
                  disabled={busy}
                  onClick={() => setRewriting(true)}
                >
                  {t("review.rewrite")}
                </Button>
              ) : (
                <Button
                  size="xs"
                  variant="light"
                  disabled={busy || item.job?.state === "pending"}
                  onClick={() => actions.write([item], narrator, null)}
                >
                  {t("review.write")}
                </Button>
              )}
              {draft && (
                <Button
                  size="xs"
                  variant="subtle"
                  color="red"
                  onClick={() => actions.discard(draft.id)}
                >
                  {t("review.discard")}
                </Button>
              )}
            </Group>
          )}
        </Stack>
      </Group>
    </article>
  );
}
