import type { TFunction } from "i18next";
import { articleEn, countedName, joinList } from "./grammar";
import type { DayFacts } from "./journal";
import { gold, type MoneyLine } from "./money";

const unique = <T>(items: T[]): T[] => [...new Set(items)];

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
  // Titled quests by name, then the untitled ones counted: "X, Y and 2 other quests".
  const quests = (titles: string[], unnamed: number) => {
    if (unnamed === 0) return list(titles);
    if (titles.length === 0) return t("page.quests", { count: unnamed });
    return list([...titles, t("page.otherQuests", { count: unnamed })]);
  };
  if (facts.accepted.length + facts.acceptedUnnamed > 0) {
    sentences.push(
      t("page.accepted", {
        list: quests(facts.accepted, facts.acceptedUnnamed),
      }),
    );
  }
  if (facts.turnedIn.length + facts.turnedInUnnamed > 0) {
    sentences.push(
      t("page.turnedIn", {
        list: quests(facts.turnedIn, facts.turnedInUnnamed),
      }),
    );
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
  if (facts.seenLoot.length > 0) {
    sentences.push(
      t("page.sawDrop", {
        list: list(facts.seenLoot.map((name) => one(name))),
      }),
    );
  }
  if (facts.loot.length > 0) {
    sentences.push(
      t("page.found", { list: list(facts.loot.map((name) => one(name))) }),
    );
  }
  for (const skill of facts.skills.filter((k) => k.profession)) {
    sentences.push(
      t("page.skillRose", {
        skill: skill.name,
        from: skill.from,
        to: skill.to,
      }),
    );
  }
  const practice = facts.skills.filter((k) => !k.profession);
  if (practice.length > 0) {
    sentences.push(
      t("page.practised", {
        list: list(
          practice.map((k) =>
            t("page.practice", { skill: k.name, from: k.from, to: k.to }),
          ),
        ),
      }),
    );
  }
  if (facts.crafted.length > 0) {
    sentences.push(
      t("page.crafted", {
        list: list(facts.crafted.map((i) => one(i.name, i.count))),
      }),
    );
  }
  // Gathered counts are loot slots, not items, so only the names are told, by gathering skill.
  const byVia = new Map<string, string[]>();
  for (const item of facts.gathered) {
    const via = item.via ?? "";
    byVia.set(via, [...(byVia.get(via) ?? []), item.name]);
  }
  for (const [via, names] of byVia) {
    sentences.push(
      t(`page.gatheredVia.${via}`, {
        list: list(names),
        defaultValue: t("page.gathered", { list: list(names) }),
      }),
    );
  }
  // Money told with where most of it came from or went, when one source holds half or more.
  const told = (side: MoneyLine[], key: "earned" | "spent") => {
    const total = side.reduce((n, m) => n + m.copper, 0);
    if (total === 0) return;
    const top = side[0];
    const mostly =
      top.copper * 2 >= total
        ? t(`page.money.${key}From.${top.source}`, { defaultValue: "" })
        : "";
    sentences.push(
      t(mostly ? `page.money.${key}Mostly` : `page.money.${key}`, {
        amount: gold(total, t),
        where: mostly,
      }),
    );
  };
  told(facts.money.earned, "earned");
  told(facts.money.spent, "spent");
  const traded = (act: string) =>
    unique(
      facts.money.auctions
        .filter((a) => a.act === act && a.name)
        .map((a) => a.name as string),
    );
  const sold = traded("sold");
  const won = traded("won");
  if (sold.length > 0)
    sentences.push(t("page.money.sold", { list: list(sold) }));
  if (won.length > 0) sentences.push(t("page.money.won", { list: list(won) }));
  if (facts.achievements.length > 0) {
    sentences.push(
      t("page.challenges", {
        count: facts.achievements.length,
        list: list(facts.achievements.map((a) => `"${a.name}"`)),
      }),
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
