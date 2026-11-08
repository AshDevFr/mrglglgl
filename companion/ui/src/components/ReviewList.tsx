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
  getSettings,
  listCharacters,
  listNarrators,
  reviewAll,
  reviewList,
  writeDrafts,
} from "@/api/commands";
import type { Journal } from "@/bindings/Journal";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { ReviewItem } from "@/bindings/ReviewItem";
import { type Problem, problem } from "@/i18n/problems";
import { dayFactRows } from "@/lib/factList";
import { formatDay } from "@/lib/format";
import { dayFacts, displayName } from "@/lib/journal";
import { narratorLabel } from "@/lib/narrators";
import { daySentences } from "@/lib/sentences";
import { scheduleRebuild } from "@/lib/siteBuild";
import { useWriting } from "@/lib/useWriting";
import { type Live, slot } from "@/lib/writing";
import { EmptyState } from "./EmptyState";
import { EntryText } from "./EntryPanel";
import { FactList } from "./FactList";

/** The longest rewrite note the writer takes; the backend cuts anything longer. */
const NOTE_CHARS = 120;

type Key = string;
const keyOf = (item: ReviewItem): Key => slot(item.kind, item.startDay);
/** A day or week of one character: the same day of two characters are two choices. */
const choiceOf = (characterId: number, item: ReviewItem): Key =>
  `${characterId}|${keyOf(item)}`;

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

/** One character's days and weeks on the list. */
interface CharacterGroup {
  characterId: number;
  items: ReviewItem[];
}

/** Finished days and weeks waiting for an entry, and the drafts waiting for the player: write
 * them, accept them, rewrite them with a note, or throw them away. One character's, or, without
 * `characterId`, every character's under their names, chosen across characters in one go. */
export function ReviewList({ characterId }: { characterId?: number }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<Set<Key>>(new Set());
  const [narrator, setNarrator] = useState<string | null>(null);
  const everyone = characterId === undefined;

  const groups = useQuery({
    queryKey: everyone ? ["review", "all"] : ["review", characterId],
    queryFn: async (): Promise<CharacterGroup[]> =>
      everyone
        ? reviewAll()
        : [{ characterId, items: await reviewList(characterId) }],
  });
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
    enabled: everyone,
  });
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
  });
  const settings = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  // Review mode decides where what is written here goes, as everywhere else.
  const direct = settings.data?.settings.writing.review === false;

  const refresh = (id: number) =>
    Promise.all(
      [["review"], ["stories", id], ["characters"], ["writing"]].map(
        (queryKey) => queryClient.invalidateQueries({ queryKey }),
      ),
    );
  const actionsFor = (id: number): Actions => ({
    write: (chosen, who, note) =>
      void writeDrafts(
        id,
        chosen.map((i) => ({ kind: i.kind, startDay: i.startDay })),
        who,
        note,
      ).then(() => refresh(id)),
    accept: (storyId) =>
      void acceptDraft(id, storyId).then(() => {
        // An accepted entry belongs on the website too.
        scheduleRebuild();
        return refresh(id);
      }),
    discard: (storyId) =>
      void discardDraft(id, storyId).then(() => refresh(id)),
  });

  if (groups.isPending) return <EmptyState title={t("loading")} />;
  const list = (groups.data ?? []).filter((g) => g.items.length > 0);
  if (list.length === 0)
    return (
      <EmptyState
        title={everyone ? t("review.emptyAll") : t("review.empty")}
        art="wave"
      />
    );

  const chosen = list
    .map((g) => ({
      characterId: g.characterId,
      items: g.items.filter((i) => selected.has(choiceOf(g.characterId, i))),
    }))
    .filter((g) => g.items.length > 0);
  const count = chosen.reduce((n, g) => n + g.items.length, 0);
  const anyDraft = chosen.some((g) => g.items.some((i) => i.drafts.length > 0));
  const toggle = (key: Key) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const each = (f: (actions: Actions, i: ReviewItem) => void) => () => {
    for (const g of chosen) {
      const actions = actionsFor(g.characterId);
      for (const i of g.items) f(actions, i);
    }
    setSelected(new Set());
  };
  const nameOf = (id: number) => {
    const summary = characters.data?.find((c) => c.character.id === id);
    return summary ? displayName(summary.character) : "";
  };

  return (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        {direct ? t("review.introDirect") : t("review.intro")}
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
          disabled={count === 0}
          onClick={() => {
            for (const g of chosen)
              actionsFor(g.characterId).write(g.items, narrator, null);
            setSelected(new Set());
          }}
        >
          {t("review.writeSelected", { count })}
        </Button>
        <Button
          size="xs"
          variant="subtle"
          disabled={!anyDraft}
          onClick={each((a, i) => i.drafts[0] && a.accept(i.drafts[0].id))}
        >
          {t("review.acceptSelected")}
        </Button>
        <Button
          size="xs"
          variant="subtle"
          color="red"
          disabled={!anyDraft}
          onClick={each((a, i) => {
            for (const d of i.drafts) a.discard(d.id);
          })}
        >
          {t("review.discardSelected")}
        </Button>
      </Group>
      {list.map((g) => (
        <CharacterItems
          key={g.characterId}
          group={g}
          name={everyone ? nameOf(g.characterId) : null}
          narrators={narrators.data ?? []}
          narrator={narrator}
          selected={selected}
          onToggle={toggle}
          actions={actionsFor(g.characterId)}
        />
      ))}
    </Stack>
  );
}

interface ItemsProps {
  group: CharacterGroup;
  /** The character's name, as a heading over their days; none on a character's own page. */
  name: string | null;
  narrators: NarratorInfo[];
  narrator: string | null;
  selected: Set<Key>;
  onToggle: (key: Key) => void;
  actions: Actions;
}

/** One character's rows, with that character's journal and what the writer streams for them. */
function CharacterItems({
  group,
  name,
  narrators,
  narrator,
  selected,
  onToggle,
  actions,
}: ItemsProps) {
  const { characterId, items } = group;
  const live = useWriting(characterId);
  const journal = useQuery({
    queryKey: ["journal", characterId],
    queryFn: () => characterJournal(characterId),
  });
  const rows = items.map((item) => {
    const key = choiceOf(characterId, item);
    return (
      <ReviewRow
        key={key}
        item={item}
        journal={journal.data ?? null}
        narrators={narrators}
        narrator={narrator}
        live={live[keyOf(item)]}
        selected={selected.has(key)}
        onToggle={() => onToggle(key)}
        actions={actions}
      />
    );
  });
  if (name === null) return <>{rows}</>;
  return (
    <section aria-label={name}>
      <Stack gap="lg">
        <h3 className="reading">
          {name} · {items.length}
        </h3>
        {rows}
      </Stack>
    </section>
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
  const facts = journal && day ? dayFacts(journal, day) : null;
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
              {/* As the journal shows the day: item by item, or the sentence saying nothing was. */}
              <FactList
                rows={facts.nothingHappened ? [] : dayFactRows(facts, t)}
                empty={daySentences(facts, t, i18n.language).join(" ")}
              />
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
