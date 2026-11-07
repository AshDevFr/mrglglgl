import type { Day } from "@/bindings/Day";

/** How a character grew in a day beyond levels and quests, from the progress events, which a
 * compacted day keeps. */
export interface ProgressFacts {
  spells: string[];
  /** Mounts and pets, by name. */
  collected: string[];
  reputations: { faction: string; standing: string }[];
  /** Better items put on, with their item level when known. */
  gear: { name: string; level: number | null }[];
  bound: string | null;
  flightPaths: string[];
  /** Duels, each with the opponent: shown here, never shared. */
  duels: { won: boolean; opponent: string | null }[];
  /** Dungeon and raid runs, with minutes inside when known. */
  instances: { name: string; minutes: number | null }[];
}

export function progressFacts(day: Day): ProgressFacts {
  const p: ProgressFacts = {
    spells: [],
    collected: [],
    reputations: [],
    gear: [],
    bound: null,
    flightPaths: [],
    duels: [],
    instances: [],
  };
  const add = (list: string[], name: string | null) => {
    if (name && !list.includes(name)) list.push(name);
  };
  for (const event of day.events ?? []) {
    const k = event.kind;
    switch (k.kind) {
      case "spell":
        add(p.spells, k.name);
        break;
      case "mount":
      case "pet":
        add(p.collected, k.name);
        break;
      case "reputation":
        if (k.faction && k.standing)
          p.reputations.push({ faction: k.faction, standing: k.standing });
        break;
      case "gear":
        if (k.name) p.gear.push({ name: k.name, level: k.itemLevel });
        break;
      case "bind":
        p.bound = k.area ?? p.bound;
        break;
      case "flightPath":
        add(p.flightPaths, k.area);
        break;
      case "duel":
        p.duels.push({ won: k.won === true, opponent: k.opponent });
        break;
      case "instance":
        if (!k.name) break;
        if (k.act === "enter" && !p.instances.some((i) => i.name === k.name))
          p.instances.push({ name: k.name, minutes: null });
        if (k.act === "leave") {
          const run = [...p.instances].reverse().find((i) => i.name === k.name);
          if (run && k.duration !== null)
            run.minutes = Math.round(k.duration / 60);
        }
        break;
      default:
        break;
    }
  }
  return p;
}

export function hasProgress(p: ProgressFacts): boolean {
  return (
    p.spells.length +
      p.collected.length +
      p.reputations.length +
      p.gear.length +
      p.flightPaths.length +
      p.duels.length +
      p.instances.length >
      0 || p.bound !== null
  );
}
