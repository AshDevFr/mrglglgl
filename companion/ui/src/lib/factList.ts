import type { TFunction } from "i18next";
import type { DayFacts } from "./journal";

/** One row of a day's facts: what kind of thing, and each thing on its own line. */
export interface FactRow {
  key: string;
  label: string;
  items: string[];
}

/** "Prairie Wolf × 5", or just the name for one. */
function counted(name: string, count: number): string {
  return count > 1 ? `${name} × ${count}` : name;
}

/** A day's facts as labelled lists, close to what the addon recorded: the raw view of the day,
 * where the sentences are its summary. Rows with nothing in them are left out. */
export function dayFactRows(facts: DayFacts, t: TFunction): FactRow[] {
  const rows: FactRow[] = [];
  const add = (key: string, items: string[]) => {
    if (items.length > 0) rows.push({ key, label: t(`facts.${key}`), items });
  };

  if (facts.levelTo !== null) {
    add("level", [
      facts.leveledUp && facts.levelFrom !== null
        ? `${facts.levelFrom} → ${facts.levelTo}`
        : String(facts.levelTo),
    ]);
  }
  add("zones", facts.zones);
  add("subzones", facts.subzones);
  add("discovered", facts.discovered);

  const untitled = (n: number) =>
    n > 0 ? [t("facts.untitledQuests", { count: n })] : [];
  add("turnedIn", [...facts.turnedIn, ...untitled(facts.turnedInUnnamed)]);
  add("accepted", [...facts.accepted, ...untitled(facts.acceptedUnnamed)]);

  add("bosses", facts.bosses);
  const notable = new Set(
    facts.notable.filter((k) => k.name !== null).map((k) => k.name),
  );
  const killed = facts.kills
    .filter((k) => k.name !== null)
    .map((k) => {
      const line = counted(k.name as string, k.count);
      return notable.has(k.name)
        ? `${line} (${t(`page.class.${k.classification}`)})`
        : line;
    });
  const unnamed = facts.kills
    .filter((k) => k.name === null)
    .reduce((n, k) => n + k.count, 0);
  if (unnamed > 0) killed.push(t("facts.unnamedKills", { count: unnamed }));
  if (facts.anonKills > 0)
    killed.push(t("facts.anonKills", { count: facts.anonKills }));
  add("kills", killed);

  add(
    "deaths",
    facts.deaths.map((d) => {
      const what = d.environment
        ? t(`page.environment.${d.environment}`, {
            defaultValue: t("page.environment.other"),
          })
        : (d.killer ?? t("facts.unknownCause"));
      return d.place ? t("facts.at", { what, place: d.place }) : what;
    }),
  );

  const catches = facts.catches.map((c) =>
    counted(c.name ?? t("facts.unknownItem"), c.count),
  );
  // Older digests count catches without listing them.
  if (catches.length === 0 && facts.fish > 0)
    catches.push(t("facts.catches", { count: facts.fish }));
  add("fishing", catches);

  add("loot", facts.loot);
  add("seenLoot", facts.seenLoot);
  add(
    "skills",
    facts.skills.map((s) => `${s.name} ${s.from} → ${s.to}`),
  );
  add(
    "crafted",
    facts.crafted.map((i) => counted(i.name, i.count)),
  );
  add(
    "gathered",
    facts.gathered.map((i) =>
      i.via
        ? `${counted(i.name, i.count)} (${i.via})`
        : counted(i.name, i.count),
    ),
  );
  add("companions", facts.companions);
  if (facts.screenshots > 0)
    add("screenshots", [
      t("facts.screenshotCount", { count: facts.screenshots }),
    ]);
  return rows;
}
