import type { CharacterSummary } from "@/bindings/CharacterSummary";
import type { Day } from "@/bindings/Day";
import type { Journal } from "@/bindings/Journal";
import type { StoredCharacter } from "@/bindings/StoredCharacter";

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
  accepted: string[];
  turnedIn: string[];
  kills: KillLine[];
  notable: KillLine[];
  murlocs: number;
  anonKills: number;
  deaths: DeathLine[];
  fish: number;
  boots: number;
  catches: CatchLine[];
  loot: string[];
  companions: string[];
  bosses: string[];
  skills: SkillLine[];
  crafted: ItemLine[];
  gathered: ItemLine[];
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

function questTitle(journal: Journal, id: number): string {
  return journal.quests[id]?.title ?? `#${id}`;
}

function place(zone: string | null, subzone: string | null): string | null {
  return subzone || zone || null;
}

/** Facts from raw events when the archive has them, from the digest otherwise. */
export function dayFacts(journal: Journal, day: Day): DayFacts {
  const facts = eventFacts(journal, day);
  const work = professions(day);
  const busy =
    work.skills.length > 0 ||
    work.crafted.length > 0 ||
    work.gathered.length > 0;
  return {
    ...facts,
    ...work,
    nothingHappened: facts.nothingHappened && !busy,
  };
}

function eventFacts(
  journal: Journal,
  day: Day,
): Omit<DayFacts, "skills" | "crafted" | "gathered"> {
  const digest = day.digest;
  if (day.events === null) {
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
      accepted: (d?.accepted ?? []).map((id) => questTitle(journal, id)),
      turnedIn: (d?.turnedIn ?? []).map((id) => questTitle(journal, id)),
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
      loot: (d?.loot ?? []).map((l) => l.name ?? `#${l.itemId}`),
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
  const facts: Omit<DayFacts, "skills" | "crafted" | "gathered"> = {
    levelFrom: null,
    levelTo: null,
    leveledUp: false,
    zones: [],
    subzones: [],
    discovered: [],
    accepted: [],
    turnedIn: [],
    kills,
    notable: kills.filter((k) => NOTABLE.has(k.classification)),
    murlocs: kills.filter((k) => k.murloc).reduce((n, k) => n + k.count, 0),
    anonKills: day.anonKills,
    deaths: [],
    fish: 0,
    boots: 0,
    catches: [],
    loot: [],
    companions: [],
    bosses: [],
    screenshots: 0,
    onlyFished: digest?.onlyFished ?? false,
    nothingHappened: digest?.nothingHappened ?? false,
    compacted: false,
  };
  const catches = new Map<string, CatchLine>();
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
        if (kind.questId !== null)
          facts.accepted.push(questTitle(journal, kind.questId));
        break;
      case "questTurnin":
        if (kind.questId !== null)
          facts.turnedIn.push(questTitle(journal, kind.questId));
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
        facts.loot.push(kind.name ?? `#${kind.itemId}`);
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
  if (!digest) {
    facts.onlyFished =
      facts.fish > 0 &&
      kills.length === 0 &&
      day.anonKills === 0 &&
      facts.accepted.length === 0 &&
      facts.turnedIn.length === 0;
    facts.nothingHappened =
      kills.length === 0 &&
      day.anonKills === 0 &&
      facts.deaths.length === 0 &&
      facts.accepted.length === 0 &&
      facts.turnedIn.length === 0 &&
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
    for (const e of [...(day.events ?? [])].reverse()) {
      snapshot.level ??= e.level;
      snapshot.zone ??= e.zone || null;
    }
    snapshot.level ??= day.digest?.levelTo ?? null;
    snapshot.zone ??= day.digest?.zones.at(-1) ?? null;
    if (snapshot.level !== null && snapshot.zone !== null) break;
  }
  return snapshot;
}
