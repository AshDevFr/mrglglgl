import type { CharacterSummary } from "@/bindings/CharacterSummary";
import type { Day } from "@/bindings/Day";
import type { Journal } from "@/bindings/Journal";
import type { StoredCharacter } from "@/bindings/StoredCharacter";
import { type MoneyFacts, moneyFacts } from "./money";

/** "Wren Hollowbrook", or the folder name until the journal has been read. */
export function displayName(character: StoredCharacter): string {
  const parts = [character.name, character.surname].filter(Boolean);
  return parts.length > 0 ? parts.join(" ") : character.characterFolder;
}

export interface RealmGroup {
  realm: string;
  characters: CharacterSummary[];
}

export interface AccountGroup {
  account: string;
  realms: RealmGroup[];
}

/** Characters grouped by account, then realm, as the sidebar lists them. */
export function groupCharacters(list: CharacterSummary[]): AccountGroup[] {
  const accounts = new Map<string, Map<string, CharacterSummary[]>>();
  for (const summary of list) {
    const c = summary.character;
    const realms = accounts.get(c.account) ?? new Map();
    accounts.set(c.account, realms);
    const realm = c.realm ?? c.realmFolder;
    realms.set(realm, [...(realms.get(realm) ?? []), summary]);
  }
  return [...accounts.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([account, realms]) => ({
      account,
      realms: [...realms.entries()]
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([realm, characters]) => ({
          realm,
          characters: [...characters].sort((a, b) =>
            displayName(a.character).localeCompare(displayName(b.character)),
          ),
        })),
    }));
}

export interface KillLine {
  name: string | null;
  npcId: number | null;
  count: number;
  classification: string;
  murloc: boolean;
}

export interface DeathLine {
  killer: string | null;
  environment: string | null;
  /** Where it happened: the subzone when there was one, else the zone. */
  place: string | null;
}

export interface SkillLine {
  name: string;
  /** A trade or secondary skill, as opposed to weapon practice and the like. */
  profession: boolean;
  from: number;
  to: number;
}

export interface ItemLine {
  name: string;
  count: number;
  /** For gathered items: the gathering skill used, e.g. "Mining". */
  via: string | null;
}

/**
 * Skill IDs of the trade and secondary skills. IDs do not depend on the client's language; the
 * skill list headings do, so the heading is only a fallback when an ID is missing.
 */
const PROFESSION_SKILLS = new Set([
  129, 164, 165, 171, 182, 185, 186, 197, 202, 333, 356, 393,
]);

function isProfession(id: number | null, category: string | null): boolean {
  return id !== null
    ? PROFESSION_SKILLS.has(id)
    : category === "Professions" || category === "Secondary Skills";
}

/** Skills, crafting and gathering: the day's own maps while it has them, else the digest. */
function professions(
  day: Day,
): Pick<DayFacts, "skills" | "crafted" | "gathered"> {
  const d = day.digest;
  const ownSkills = Object.entries(day.skills ?? {});
  const skills: SkillLine[] = (
    ownSkills.length > 0
      ? ownSkills.map(([id, s]) => ({
          name: s.name ?? "",
          profession: isProfession(Number(id), s.category),
          from: s.from,
          to: s.to,
        }))
      : (d?.skills ?? []).map((s) => ({
          name: s.name ?? "",
          profession: isProfession(s.skillId, s.category),
          from: s.from,
          to: s.to,
        }))
  )
    .filter((s) => s.name !== "" && s.to > s.from)
    // The game lists some professions under two skill IDs with one name and rank, only one of
    // them a known profession ID: one line per name, a profession if either says so.
    .reduce<SkillLine[]>((merged, s) => {
      const same = merged.find((m) => m.name === s.name);
      if (same) {
        same.profession ||= s.profession;
        same.from = Math.min(same.from, s.from);
        same.to = Math.max(same.to, s.to);
      } else merged.push({ ...s });
      return merged;
    }, [])
    // Stable: professions first, each group in skill ID order.
    .sort((a, b) => Number(b.profession) - Number(a.profession));
  const items = (
    own: Record<
      string,
      { name: string | null; count: number; via: string | null }
    >,
    digested:
      | { name: string | null; count: number; via: string | null }[]
      | undefined,
  ): ItemLine[] => {
    const list =
      Object.keys(own).length > 0 ? Object.values(own) : (digested ?? []);
    return list
      .filter((i) => i.name !== null)
      .map((i) => ({ name: i.name as string, count: i.count, via: i.via }));
  };
  return {
    skills,
    crafted: items(day.crafted ?? {}, d?.crafted),
    gathered: items(day.gathered ?? {}, d?.gathered),
  };
}

export interface CatchLine {
  name: string | null;
  itemId: number | null;
  count: number;
  junk: boolean;
  boot: boolean;
}

/** What happened on one day, in the shape the day view shows it. */
export interface DayFacts {
  levelFrom: number | null;
  levelTo: number | null;
  leveledUp: boolean;
  zones: string[];
  subzones: string[];
  discovered: string[];
  /** Titles of quests taken on and not finished the same day. */
  accepted: string[];
  /** Quests taken on whose title the journal never learned. */
  acceptedUnnamed: number;
  /** Titles of quests handed in. */
  turnedIn: string[];
  /** Quests handed in whose title the journal never learned, as when the quest was taken before
   * the addon recorded quests. */
  turnedInUnnamed: number;
  /** Each titled quest's level and kind as players write them, by title: "[9]", "[9+]", "[18D]". */
  questTags: Record<string, string>;
  kills: KillLine[];
  notable: KillLine[];
  murlocs: number;
  anonKills: number;
  deaths: DeathLine[];
  fish: number;
  boots: number;
  catches: CatchLine[];
  loot: string[];
  /** Rare items seen dropping in a group loot window, which may have gone to someone else. */
  seenLoot: string[];
  companions: string[];
  bosses: string[];
  skills: SkillLine[];
  crafted: ItemLine[];
  gathered: ItemLine[];
  /** Money earned and spent by source, the balance, and auction house events. */
  money: MoneyFacts;
  screenshots: number;
  onlyFished: boolean;
  nothingHappened: boolean;
  /** Only the digest survived: the game had already compacted the day. */
  compacted: boolean;
}

const NOTABLE = new Set(["elite", "rare", "rareelite", "worldboss"]);

function unique<T>(items: T[]): T[] {
  return [...new Set(items)];
}

function companionName(who: string): string {
  return who.replace("-", " ");
}

/** The day's quests by title: a quest handed in the same day it was taken is told once, as
 * finished, and quests without a known title are counted rather than shown as numbers. */
function questFacts(
  journal: Journal,
  acceptedIds: number[],
  turnedInIds: number[],
): Pick<
  DayFacts,
  "accepted" | "acceptedUnnamed" | "turnedIn" | "turnedInUnnamed" | "questTags"
> {
  const done = unique(turnedInIds);
  const taken = unique(acceptedIds).filter((id) => !done.includes(id));
  const split = (ids: number[]) => {
    const titles: string[] = [];
    let unnamed = 0;
    for (const id of ids) {
      const title = journal.quests[id]?.title;
      if (title) titles.push(title);
      else unnamed += 1;
    }
    return { titles: unique(titles), unnamed };
  };
  const a = split(taken);
  const t = split(done);
  const questTags: Record<string, string> = {};
  for (const id of [...taken, ...done]) {
    const quest = journal.quests[id];
    const tag = quest ? questTag(quest.level, quest.kind) : null;
    if (quest?.title && tag) questTags[quest.title] = tag;
  }
  return {
    accepted: a.titles,
    acceptedUnnamed: a.unnamed,
    turnedIn: t.titles,
    turnedInUnnamed: t.unnamed,
    questTags,
  };
}

const KIND_MARK: Record<string, string> = {
  elite: "+",
  dungeon: "D",
  raid: "R",
  group: "G",
};

/** A quest's level and kind the way players write them: [9], [9+] elite, [18D] dungeon, [40R]
 * raid, [12G] group. Nothing without a level. */
export function questTag(
  level: number | null | undefined,
  kind: string | null | undefined,
): string | null {
  if (level === null || level === undefined) return null;
  return `[${level}${(kind && KIND_MARK[kind]) ?? ""}]`;
}

function place(zone: string | null, subzone: string | null): string | null {
  return subzone || zone || null;
}

/** Facts from raw events when the archive has them, from the digest otherwise. */
export function dayFacts(journal: Journal, day: Day): DayFacts {
  const facts = eventFacts(journal, day);
  const work = professions(day);
  const money = moneyFacts(day);
  const busy =
    work.skills.length > 0 ||
    work.crafted.length > 0 ||
    work.gathered.length > 0 ||
    money.earned.length > 0 ||
    money.spent.length > 0 ||
    money.auctions.length > 0;
  return {
    ...facts,
    ...work,
    money,
    nothingHappened: facts.nothingHappened && !busy,
  };
}

function eventFacts(
  journal: Journal,
  day: Day,
): Omit<DayFacts, "skills" | "crafted" | "gathered" | "money"> {
  const digest = day.digest;
  // A compacted day keeps a few key events; its digest is what the whole day was.
  if (day.events === null || day.compacted) {
    const d = digest;
    const kills: KillLine[] = (d?.kills ?? []).map((k) => ({
      name: k.name,
      npcId: k.npcId,
      count: k.count,
      classification: k.classification,
      murloc: k.murloc,
    }));
    return {
      levelFrom: d?.levelFrom ?? null,
      levelTo: d?.levelTo ?? null,
      leveledUp: (d?.counts.levels ?? 0) > 0,
      zones: d?.zones ?? [],
      subzones: d?.subzones ?? [],
      discovered: d?.discovered ?? [],
      ...questFacts(journal, d?.accepted ?? [], d?.turnedIn ?? []),
      kills,
      notable: kills.filter((k) => NOTABLE.has(k.classification)),
      murlocs: d?.counts.murlocs ?? 0,
      // A digest lists only the main kills; the rest are counted as nameless foes.
      anonKills:
        (d?.counts.anonKills ?? 0) +
        Math.max(
          0,
          (d?.counts.kills ?? 0) -
            (d?.counts.anonKills ?? 0) -
            kills.reduce((n, k) => n + k.count, 0),
        ),
      deaths: (d?.deaths ?? []).map((death) => ({
        killer: death.killer?.name ?? null,
        environment: death.environment,
        place: place(death.zone, death.subzone),
      })),
      fish: d?.counts.fish ?? 0,
      boots: d?.counts.boots ?? 0,
      catches: (d?.fish ?? []).map((f) => ({
        name: f.name,
        itemId: f.itemId,
        count: f.count,
        junk: f.junk === true,
        boot: f.boot === true,
      })),
      loot: (d?.loot ?? [])
        .filter((l) => !l.seen)
        .map((l) => l.name ?? `#${l.itemId}`),
      seenLoot: (d?.loot ?? [])
        .filter((l) => l.seen)
        .map((l) => l.name ?? `#${l.itemId}`),
      companions: (d?.companions ?? []).map(companionName),
      bosses: d?.bosses ?? [],
      screenshots: d?.counts.screenshots ?? 0,
      onlyFished: d?.onlyFished ?? false,
      nothingHappened: d?.nothingHappened ?? false,
      compacted: true,
    };
  }

  const kills: KillLine[] = Object.entries(day.kills)
    .map(([npc, k]) => ({
      name: k.name,
      npcId: Number(npc),
      count: k.count,
      classification: k.classification,
      murloc: k.murloc,
    }))
    .sort((a, b) => b.count - a.count || (a.npcId ?? 0) - (b.npcId ?? 0));
  const facts: Omit<DayFacts, "skills" | "crafted" | "gathered" | "money"> = {
    levelFrom: null,
    levelTo: null,
    leveledUp: false,
    zones: [],
    subzones: [],
    discovered: [],
    accepted: [],
    acceptedUnnamed: 0,
    turnedIn: [],
    turnedInUnnamed: 0,
    questTags: {},
    kills,
    notable: kills.filter((k) => NOTABLE.has(k.classification)),
    murlocs: kills.filter((k) => k.murloc).reduce((n, k) => n + k.count, 0),
    anonKills: day.anonKills,
    deaths: [],
    fish: 0,
    boots: 0,
    catches: [],
    loot: [],
    seenLoot: [],
    companions: [],
    bosses: [],
    screenshots: 0,
    onlyFished: digest?.onlyFished ?? false,
    nothingHappened: digest?.nothingHappened ?? false,
    compacted: false,
  };
  const catches = new Map<string, CatchLine>();
  const acceptedIds: number[] = [];
  const turnedInIds: number[] = [];
  for (const event of day.events) {
    if (event.level !== null) {
      facts.levelFrom = Math.min(facts.levelFrom ?? event.level, event.level);
      facts.levelTo = Math.max(facts.levelTo ?? event.level, event.level);
    }
    if (event.zone) facts.zones = unique([...facts.zones, event.zone]);
    if (event.subzone)
      facts.subzones = unique([...facts.subzones, event.subzone]);
    const kind = event.kind;
    switch (kind.kind) {
      case "level":
        facts.leveledUp = true;
        break;
      case "discover":
        if (kind.area) facts.discovered.push(kind.area);
        break;
      case "questAccept":
        if (kind.questId !== null) acceptedIds.push(kind.questId);
        break;
      case "questTurnin":
        if (kind.questId !== null) turnedInIds.push(kind.questId);
        break;
      case "death":
        facts.deaths.push({
          killer: kind.killer?.name ?? null,
          environment: kind.environment,
          place: place(event.zone, event.subzone),
        });
        break;
      case "fish": {
        facts.fish += 1;
        if (kind.boot) facts.boots += 1;
        const key = String(kind.itemId ?? kind.name);
        const line = catches.get(key) ?? {
          name: kind.name,
          itemId: kind.itemId,
          count: 0,
          junk: kind.junk === true,
          boot: kind.boot === true,
        };
        line.count += 1;
        catches.set(key, line);
        break;
      }
      case "loot":
        (kind.seen ? facts.seenLoot : facts.loot).push(
          kind.name ?? `#${kind.itemId}`,
        );
        break;
      case "groupJoin":
        if (kind.who) facts.companions.push(companionName(kind.who));
        break;
      case "boss":
        if (kind.name) facts.bosses.push(kind.name);
        break;
      case "screenshot":
        facts.screenshots += 1;
        break;
      default:
        break;
    }
  }
  facts.catches = [...catches.values()];
  Object.assign(facts, questFacts(journal, acceptedIds, turnedInIds));
  if (!digest) {
    facts.onlyFished =
      facts.fish > 0 &&
      kills.length === 0 &&
      day.anonKills === 0 &&
      acceptedIds.length === 0 &&
      turnedInIds.length === 0;
    facts.nothingHappened =
      kills.length === 0 &&
      day.anonKills === 0 &&
      facts.deaths.length === 0 &&
      acceptedIds.length === 0 &&
      turnedInIds.length === 0 &&
      facts.fish === 0 &&
      facts.loot.length === 0 &&
      !facts.leveledUp &&
      facts.discovered.length === 0 &&
      facts.bosses.length === 0 &&
      facts.companions.length === 0;
  }
  return facts;
}

/** Days, newest first. */
export function daysNewestFirst(journal: Journal): string[] {
  return Object.keys(journal.days).sort().reverse();
}

/** The latest level and zone the journal knows, for the character header. */
export interface Snapshot {
  level: number | null;
  zone: string | null;
}

export function latestSnapshot(journal: Journal): Snapshot {
  const snapshot: Snapshot = { level: null, zone: null };
  for (const date of daysNewestFirst(journal)) {
    const day = journal.days[date];
    // A level never goes down, but the game can stamp the logout with the level from before
    // the session's level-ups: the newest day's highest level is the current one.
    const levels = (day.events ?? [])
      .map((e) => e.level)
      .filter((l): l is number => l !== null);
    if (levels.length > 0) snapshot.level ??= Math.max(...levels);
    for (const e of [...(day.events ?? [])].reverse()) {
      snapshot.zone ??= e.zone || null;
    }
    snapshot.level ??= day.digest?.levelTo ?? null;
    snapshot.zone ??= day.digest?.zones.at(-1) ?? null;
    if (snapshot.level !== null && snapshot.zone !== null) break;
  }
  return snapshot;
}
