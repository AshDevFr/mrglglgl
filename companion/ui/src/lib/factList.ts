import type { TFunction } from "i18next";
import type { DayFacts } from "./journal";

/** One thing in a row: its text, and the name to look up when names link to Wowhead. */
export interface FactItem {
  text: string;
  /** The name alone (no count, level or place), for a Wowhead search; none for counts. */
  name?: string;
}

/** One row of a day's facts: what kind of thing, and each thing on its own line. */
export interface FactRow {
  key: string;
  label: string;
  items: FactItem[];
}

/** "Prairie Wolf × 5", or just the name for one. */
function counted(name: string, count: number): string {
  return count > 1 ? `${name} × ${count}` : name;
}

/** A day's facts as labelled lists, close to what the addon recorded: the raw view of the day,
 * where the sentences are its summary. Rows with nothing in them are left out. */
export function dayFactRows(facts: DayFacts, t: TFunction): FactRow[] {
  const rows: FactRow[] = [];
  const add = (key: string, items: FactItem[]) => {
    if (items.length > 0) rows.push({ key, label: t(`facts.${key}`), items });
  };
  /** A thing worth looking up: its text, searched by its bare name. */
  const named = (name: string, text = name): FactItem => ({ text, name });
  const plain = (text: string): FactItem => ({ text });

  if (facts.levelTo !== null) {
    add("level", [
      plain(
        facts.leveledUp && facts.levelFrom !== null
          ? `${facts.levelFrom} → ${facts.levelTo}`
          : String(facts.levelTo),
      ),
    ]);
  }
  add(
    "zones",
    facts.zones.map((z) => named(z)),
  );
  add(
    "subzones",
    facts.subzones.map((z) => named(z)),
  );
  add(
    "discovered",
    facts.discovered.map((z) => named(z)),
  );

  const quest = (title: string) => {
    const tag = facts.questTags[title];
    return named(title, tag ? `${title} ${tag}` : title);
  };
  const untitled = (n: number) =>
    n > 0 ? [plain(t("facts.untitledQuests", { count: n }))] : [];
  add("turnedIn", [
    ...facts.turnedIn.map(quest),
    ...untitled(facts.turnedInUnnamed),
  ]);
  add("accepted", [
    ...facts.accepted.map(quest),
    ...untitled(facts.acceptedUnnamed),
  ]);

  add(
    "bosses",
    facts.bosses.map((b) => named(b)),
  );
  const notable = new Set(
    facts.notable.filter((k) => k.name !== null).map((k) => k.name),
  );
  const killed: FactItem[] = facts.kills
    .filter((k) => k.name !== null)
    .map((k) => {
      const name = k.name as string;
      const line = counted(name, k.count);
      return named(
        name,
        notable.has(name)
          ? `${line} (${t(`page.class.${k.classification}`)})`
          : line,
      );
    });
  const unnamed = facts.kills
    .filter((k) => k.name === null)
    .reduce((n, k) => n + k.count, 0);
  if (unnamed > 0)
    killed.push(plain(t("facts.unnamedKills", { count: unnamed })));
  if (facts.anonKills > 0)
    killed.push(plain(t("facts.anonKills", { count: facts.anonKills })));
  add("kills", killed);

  add(
    "deaths",
    facts.deaths.map((d) => {
      const what = d.environment
        ? t(`page.environment.${d.environment}`, {
            defaultValue: t("page.environment.other"),
          })
        : (d.killer ?? t("facts.unknownCause"));
      const text = d.place ? t("facts.at", { what, place: d.place }) : what;
      return d.killer && !d.environment ? named(d.killer, text) : plain(text);
    }),
  );

  const catches: FactItem[] = facts.catches.map((c) =>
    c.name
      ? named(c.name, counted(c.name, c.count))
      : plain(counted(t("facts.unknownItem"), c.count)),
  );
  // Older digests count catches without listing them.
  if (catches.length === 0 && facts.fish > 0)
    catches.push(plain(t("facts.catches", { count: facts.fish })));
  add("fishing", catches);

  add(
    "loot",
    facts.loot.map((i) => named(i)),
  );
  add(
    "seenLoot",
    facts.seenLoot.map((i) => named(i)),
  );
  add(
    "skills",
    facts.skills.map((s) => named(s.name, `${s.name} ${s.from} → ${s.to}`)),
  );
  add(
    "crafted",
    facts.crafted.map((i) => named(i.name, counted(i.name, i.count))),
  );
  add(
    "gathered",
    facts.gathered.map((i) =>
      named(
        i.name,
        i.via
          ? `${counted(i.name, i.count)} (${i.via})`
          : counted(i.name, i.count),
      ),
    ),
  );
  // Other players: their names are theirs, not something to look up.
  add(
    "companions",
    facts.companions.map((c) => plain(c)),
  );
  if (facts.screenshots > 0)
    add("screenshots", [
      plain(t("facts.screenshotCount", { count: facts.screenshots })),
    ]);
  return rows;
}
