import i18n from "i18next";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  characterJournal,
  characterMilestones,
  characterScreenshots,
  characterStories,
  listCharacters,
  siteStandInNames,
} from "@/api/commands";
import type { Journal } from "@/bindings/Journal";
import type { SiteCharacter } from "@/bindings/SiteCharacter";
import type { SiteData } from "@/bindings/SiteData";
import type { SiteDay } from "@/bindings/SiteDay";
import type { SitePost } from "@/bindings/SitePost";
import type { SiteSettings } from "@/bindings/SiteSettings";
import type { SiteStamp } from "@/bindings/SiteStamp";
import type { Story } from "@/bindings/Story";
import { EntryText } from "@/components/EntryPanel";
import { InkStamp } from "@/components/InkStamp";
import { photoCaption } from "@/lib/captions";
import { formatDay } from "@/lib/format";
import { dayFacts, displayName, latestSnapshot } from "@/lib/journal";
import { savedPaper } from "@/lib/paper";
import { daySentences } from "@/lib/sentences";
import { dayKey, weekStart } from "@/lib/weeks";
import { versionsOf } from "@/lib/writing";

/** The words of the site's default theme, in the app's language. */
const LABELS = [
  "characters",
  "posts",
  "archive",
  "milestones",
  "feed",
  "latest",
  "newer",
  "older",
  "previous_week",
  "next_week",
  "no_posts",
  "no_milestones",
  "light_dark",
  "change_paper",
  "written_with",
];

/** "2026-w45": the ISO week of a Monday, as a post's folder name. */
export function weekSlug(monday: string): string {
  const date = new Date(`${monday}T00:00:00Z`);
  const thursday = new Date(date);
  thursday.setUTCDate(date.getUTCDate() + 3);
  const year = thursday.getUTCFullYear();
  const first = new Date(Date.UTC(year, 0, 4));
  const week =
    1 +
    Math.round(
      ((thursday.getTime() - first.getTime()) / 86400000 -
        3 +
        ((first.getUTCDay() + 6) % 7)) /
        7,
    );
  return `${year}-w${String(week).padStart(2, "0")}`;
}

function slugify(text: string): string {
  return (
    text
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "character"
  );
}

function titleCase(word: string | null): string {
  if (!word) return "";
  return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
}

/** Other players as the journal recorded them: "Tamsin-Reedwater" is Tamsin, of Reedwater. */
function companions(journal: Journal): string[] {
  const who = new Set<string>(Object.keys(journal.companions ?? {}));
  for (const day of Object.values(journal.days)) {
    for (const e of day.events ?? []) {
      if (e.kind.kind === "groupJoin" && e.kind.who) who.add(e.kind.who);
    }
    for (const c of day.digest?.companions ?? []) who.add(c);
  }
  return [...who];
}

/** Word replacements for other players: stand-in names, or their names without their realm. */
function replacements(
  site: SiteSettings,
  journal: Journal,
  aliases: Map<string, string>,
): [RegExp, string][] {
  const out: [RegExp, string][] = [];
  const word = (text: string) =>
    new RegExp(
      `(?<![\\p{L}\\p{N}])${text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?![\\p{L}\\p{N}])`,
      "gu",
    );
  for (const who of companions(journal)) {
    const [name, ...realm] = who.split("-");
    const target = site.standInNames ? (aliases.get(name) ?? "Someone") : name;
    if (!site.standInNames && !site.hideRealm) continue;
    if (realm.length > 0) {
      out.push([word(who), target]);
      out.push([word(`${name} ${realm.join(" ")}`), target]);
    }
    if (site.standInNames) out.push([word(name), target]);
  }
  return out;
}

/** The site's content for its characters, as the core renders it: finished days only, accepted
 * entries only (never one told so far), other players under stand-in names when asked. */
export async function buildSiteData(
  site: SiteSettings,
  { today = dayKey(new Date()) }: { today?: string } = {},
): Promise<SiteData> {
  const t = i18n.t.bind(i18n);
  const lang = i18n.language;
  const summaries = await listCharacters();
  const aliases = new Map(
    site.standInNames
      ? (await siteStandInNames(site.characters)).map((a) => [a.real, a.alias])
      : [],
  );
  const slugs = new Set<string>();
  const characters: SiteCharacter[] = [];

  for (const id of site.characters) {
    const summary = summaries.find((c) => c.character.id === id);
    const [journal, stories, awards, shots] = await Promise.all([
      characterJournal(id),
      characterStories(id),
      characterMilestones(id),
      characterScreenshots(id),
    ]);
    if (!summary || !journal) continue;
    const swaps = replacements(site, journal, aliases);
    const clean = (text: string) =>
      swaps.reduce((acc, [from, to]) => acc.replace(from, to), text);
    const told = (kind: Story["kind"], start: string) =>
      versionsOf(stories, kind, start).find(
        (s) => s.check !== "failed" && !s.partial,
      );
    const entryHtml = (text: string) =>
      renderToStaticMarkup(createElement(EntryText, { text: clean(text) }));
    const stamp = (a: (typeof awards)[number]): SiteStamp => ({
      name: t(`milestones.${a.id}.name`),
      description: t(`milestones.${a.id}.description`),
      day: a.day,
      svg: renderToStaticMarkup(
        createElement(InkStamp, { award: a, size: 96 }),
      ),
    });

    const days = Object.keys(journal.days)
      .filter((d) => d < today)
      .sort();
    const weeks = new Map<string, string[]>();
    for (const day of days) {
      const monday = weekStart(day);
      weeks.set(monday, [...(weeks.get(monday) ?? []), day]);
    }
    const posts: SitePost[] = [...weeks].map(([monday, weekDays]) => {
      const siteDays: SiteDay[] = weekDays.map((day) => {
        const entry = told("day", day);
        const facts = clean(
          daySentences(dayFacts(journal, journal.days[day]), t, lang).join(" "),
        );
        return {
          date: day,
          title: formatDay(day, lang, true),
          html: entry
            ? entryHtml(entry.text)
            : renderToStaticMarkup(
                createElement("p", { className: "entry" }, facts),
              ),
          written: entry !== undefined,
          stamps: awards.filter((a) => a.day === day).map(stamp),
          photos: (shots[day] ?? []).map((s, i) => ({
            source: s.path,
            file: `${day}-${i + 1}.jpg`,
            caption: clean(photoCaption(t, lang, journal, day, s)),
          })),
        };
      });
      const chapter = told("week", monday);
      const firstText =
        chapter?.text ??
        weekDays.map((d) => told("day", d)?.text).find(Boolean) ??
        daySentences(
          dayFacts(journal, journal.days[weekDays[0]]),
          t,
          lang,
        ).join(" ");
      const excerpt = clean(firstText.replace(/\s+/g, " ").trim());
      const date = new Date(`${monday}T00:00:00`);
      return {
        slug: weekSlug(monday),
        title: t("export.weekOf", { date: formatDay(monday, lang, true) }),
        date: monday,
        month: monday.slice(0, 7),
        month_label: new Intl.DateTimeFormat(lang, {
          month: "long",
          year: "numeric",
        }).format(date),
        excerpt: excerpt.length > 220 ? `${excerpt.slice(0, 217)}…` : excerpt,
        chapter_html: chapter ? entryHtml(chapter.text) : null,
        days: siteDays,
      };
    });

    let slug = slugify(displayName(summary.character));
    for (let n = 2; slugs.has(slug); n++) slug = `${slug}-${n}`;
    slugs.add(slug);
    const c = summary.character;
    const level = latestSnapshot(journal).level;
    const who = [c.race, titleCase(c.class)].filter(Boolean).join(" ");
    characters.push({
      slug,
      name: displayName(c),
      summary:
        level === null
          ? who
          : t("site.summary", {
              who,
              level,
              defaultValue: `${who}, level ${level}`,
            }),
      posts,
      milestones: awards.map(stamp),
    });
  }

  return {
    title: site.title.trim() || t("site.defaultTitle"),
    language: lang,
    base_url: site.baseUrl,
    paper: savedPaper(),
    labels: Object.fromEntries(LABELS.map((k) => [k, t(`site.labels.${k}`)])),
    characters,
  };
}
