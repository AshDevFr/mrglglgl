import {
  CloseButton,
  Stack,
  Text,
  TextInput,
  UnstyledButton,
} from "@mantine/core";
import type { TFunction } from "i18next";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { Journal } from "@/bindings/Journal";
import type { Story } from "@/bindings/Story";
import { formatDay } from "@/lib/format";
import { dayFacts, daysNewestFirst } from "@/lib/journal";
import { SEARCH_LIMIT, type SearchDoc, searchJournal } from "@/lib/search";
import { daySentences } from "@/lib/sentences";

/** The field above a character's journal. ⌘ or Ctrl with F puts the cursor in it, as a page
 * search would; Escape empties it. */
export function SearchField({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation();
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (
        (e.metaKey || e.ctrlKey) &&
        !e.altKey &&
        e.key.toLowerCase() === "f"
      ) {
        e.preventDefault();
        input.current?.focus();
        input.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return (
    <TextInput
      ref={input}
      type="search"
      aria-label={t("search.label")}
      placeholder={t("search.placeholder")}
      value={value}
      onChange={(e) => onChange(e.currentTarget.value)}
      onKeyDown={(e) => {
        if (e.key === "Escape") onChange("");
      }}
      rightSection={
        value !== "" ? (
          <CloseButton
            size="sm"
            aria-label={t("search.clear")}
            onClick={() => onChange("")}
          />
        ) : undefined
      }
      maw={420}
    />
  );
}

/** What the journal page shows of each day and week, as the search reads it: the newest entry
 * kept in the journal (drafts waiting for review and entries set aside are not), and the day's
 * notes in the sentences What happened uses. */
export function searchDocs(
  journal: Journal,
  stories: Story[],
  t: TFunction,
  language: string,
): SearchDoc[] {
  const newest = new Map<string, Story>();
  for (const s of stories) {
    if (s.status !== "accepted" || s.check === "failed") continue;
    const key = `${s.kind}-${s.startDay}`;
    const seen = newest.get(key);
    if (!seen || s.id > seen.id) newest.set(key, s);
  }
  const docs: SearchDoc[] = [...newest.values()].map((s) => ({
    day: s.startDay,
    kind: s.kind === "week" ? "week" : "day",
    source: "entry",
    text: s.text,
  }));
  for (const day of daysNewestFirst(journal)) {
    docs.push({
      day,
      kind: "day",
      source: "facts",
      text: daySentences(
        dayFacts(journal, journal.days[day]),
        t,
        language,
      ).join(" "),
    });
  }
  return docs;
}

/** The days and weeks that mention the query; choosing one opens it. */
export function SearchResults({
  journal,
  stories,
  query,
  onOpen,
}: {
  journal: Journal;
  stories: Story[];
  query: string;
  onOpen: (day: string) => void;
}) {
  const { t, i18n } = useTranslation();
  // Built once per journal, not on every keystroke: the notes of every day are worded here.
  const docs = useMemo(
    () => searchDocs(journal, stories, t, i18n.language),
    [journal, stories, t, i18n.language],
  );
  const hits = searchJournal(docs, query);
  const short = query.trim().length < 2;
  const source = (doc: SearchDoc) =>
    doc.kind === "week"
      ? t("search.source.chapter")
      : t(`search.source.${doc.source}`);
  return (
    <Stack gap="xs" className="search-results">
      <Text size="sm" c="dimmed" role="status">
        {short
          ? t("search.short")
          : hits.length === 0
            ? t("search.none", { query: query.trim() })
            : `${t("search.found", { count: hits.length })}${
                hits.length === SEARCH_LIMIT
                  ? ` ${t("search.limited", { count: SEARCH_LIMIT })}`
                  : ""
              }`}
      </Text>
      {hits.map((hit) => (
        <UnstyledButton
          key={`${hit.doc.kind}-${hit.doc.source}-${hit.doc.day}`}
          className="search-hit"
          onClick={() => onOpen(hit.doc.day)}
        >
          <Text size="sm" fw={600}>
            {formatDay(hit.doc.day, i18n.language, true)}
            <Text span size="xs" c="dimmed" fw={400}>
              {" · "}
              {source(hit.doc)}
            </Text>
          </Text>
          <Text className="search-snippet">
            {hit.before}
            <mark>{hit.match}</mark>
            {hit.after}
          </Text>
        </UnstyledButton>
      ))}
    </Stack>
  );
}
