import type { TFunction } from "i18next";
import { articleEn, countedName, joinList } from "./grammar";
import type { DayFacts } from "./journal";

/** A day's facts as short plain sentences, in the order the day is told. */
export function daySentences(
  facts: DayFacts,
  t: TFunction,
  language: string,
): string[] {
  const list = (items: string[]) => joinList(language, items);
  const one = (name: string, count = 1) => countedName(language, name, count);
  const sentences: string[] = [];

  if (facts.zones.length === 1) {
    sentences.push(t("page.stayedIn", { place: facts.zones[0] }));
  } else if (facts.zones.length > 1) {
    sentences.push(t("page.travelled", { list: list(facts.zones) }));
  }
  if (facts.discovered.length > 0) {
    sentences.push(t("page.discovered", { list: list(facts.discovered) }));
  }
  if (facts.leveledUp && facts.levelTo !== null) {
    sentences.push(t("page.levelUp", { level: facts.levelTo }));
  }
  if (facts.accepted.length > 0) {
    sentences.push(t("page.accepted", { list: list(facts.accepted) }));
  }
  if (facts.turnedIn.length > 0) {
    sentences.push(t("page.turnedIn", { list: list(facts.turnedIn) }));
  }

  const named = facts.kills.filter((k) => k.name !== null);
  const unnamed = facts.kills
    .filter((k) => k.name === null)
    .reduce((n, k) => n + k.count, 0);
  const killed = named.map((k) => one(k.name as string, k.count));
  if (unnamed > 0) killed.push(t("page.unnamedKills", { count: unnamed }));
  if (killed.length > 0)
    sentences.push(t("page.killed", { list: list(killed) }));
  const notable = facts.notable.filter((k) => k.name !== null);
  if (notable.length > 0) {
    const items = notable.map((k) => {
      const kind = t(`page.class.${k.classification}`);
      const phrase = `${kind} ${k.name}`;
      return language === "en" ? `${articleEn(kind)} ${phrase}` : phrase;
    });
    sentences.push(
      t("page.notable", { count: items.length, list: list(items) }),
    );
  }
  if (facts.anonKills > 0) {
    sentences.push(t("page.anonKills", { count: facts.anonKills }));
  }

  for (const death of facts.deaths) {
    const where = death.place ? "" : "NoPlace";
    if (death.environment) {
      const what = t(`page.environment.${death.environment}`, {
        defaultValue: t("page.environment.other"),
      });
      sentences.push(t(`page.deathEnv${where}`, { what, place: death.place }));
    } else if (death.killer) {
      sentences.push(
        t(`page.diedTo${where}`, {
          killer: one(death.killer),
          place: death.place,
        }),
      );
    } else {
      sentences.push(t(`page.died${where}`, { place: death.place }));
    }
  }

  if (facts.fish > 0) {
    const fishCount = facts.catches
      .filter((c) => !c.junk)
      .reduce((n, c) => n + c.count, 0);
    const caught: string[] = [];
    if (fishCount > 0) caught.push(t("page.fish", { count: fishCount }));
    for (const c of facts.catches.filter((c) => c.junk)) {
      caught.push(
        c.name ? one(c.name, c.count) : t("page.junk", { count: c.count }),
      );
    }
    // Older digests may count catches without listing them.
    if (caught.length === 0) caught.push(t("page.fish", { count: facts.fish }));
    sentences.push(t("page.caught", { list: list(caught) }));
    if (facts.boots > 0)
      sentences.push(t("page.keptBoot", { count: facts.boots }));
  }
  if (facts.loot.length > 0) {
    sentences.push(
      t("page.found", { list: list(facts.loot.map((name) => one(name))) }),
    );
  }
  if (facts.companions.length > 0) {
    sentences.push(t("page.grouped", { list: list(facts.companions) }));
  }
  if (facts.bosses.length > 0) {
    sentences.push(t("page.defeated", { list: list(facts.bosses) }));
  }
  if (facts.onlyFished) sentences.push(t("page.onlyFished"));

  if (facts.nothingHappened || sentences.length === 0)
    return [t("page.nothing")];
  return sentences;
}
