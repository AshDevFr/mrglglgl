// The built-in narrator made for each playable race, keyed by the race token the addon records
// (the game's own, the same in every language: "NightElf", "Scourge" for the Forsaken).

const BY_RACE: Record<string, string> = {
  Human: "pompous_noble",
  Dwarf: "grumpy_dwarf",
  NightElf: "silly_elf",
  Gnome: "gnome_tinkerer",
  Orc: "orc_grunt",
  Scourge: "forsaken_chronicler",
  Undead: "forsaken_chronicler",
  Tauren: "tauren_elder",
  Troll: "easygoing_troll",
};

/** The narrator made for a race, if there is one. */
export function narratorForRace(
  race: string | null | undefined,
): string | undefined {
  return race ? BY_RACE[race.replace(/\s+/g, "")] : undefined;
}
