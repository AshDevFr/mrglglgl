// The demo's answers: the sample journal the tests use, and entries written for it in the
// narrators' voices. Everything here is made up; nothing comes from a real character.
import type { AddonView } from "@/bindings/AddonView";
import type { Award } from "@/bindings/Award";
import type { CharacterSummary } from "@/bindings/CharacterSummary";
import type { Journal } from "@/bindings/Journal";
import type { ModelsView } from "@/bindings/ModelsView";
import type { Narrator } from "@/bindings/Narrator";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { ReleaseCheck } from "@/bindings/ReleaseCheck";
import type { SettingsView } from "@/bindings/SettingsView";
import type { Story } from "@/bindings/Story";
import type { WritingStatus } from "@/bindings/WritingStatus";
// The sample journal the tests use, written by a Rust test from the shared addon fixtures.
import sampleAwards from "@/test/data/synthetic-awards.json";
import sampleJournal from "@/test/data/synthetic-journal.json";

export const journal = sampleJournal as unknown as Journal;
export const awards = sampleAwards as Award[];
const syntheticAwards = awards;

const FOLDER = "/Applications/World of Warcraft";

export const characters: CharacterSummary[] = [
  {
    character: {
      id: 1,
      flavor: "_forever_",
      account: "ACCOUNT1",
      realmFolder: "93",
      characterFolder: "Wren-Hollowbrook",
      journalPath: `${FOLDER}/_forever_/WTF/Account/ACCOUNT1/93/Wren-Hollowbrook/SavedVariables/Mrglglgl.lua`,
      name: "Wren",
      surname: "Hollowbrook",
      realm: "Test Realm",
      race: "Tauren",
      class: "HUNTER",
      faction: "Horde",
      locale: "enUS",
      lastSynced: 1794304800,
      status: null,
    },
    archived: false,
    milestones: syntheticAwards.length,
    toReview: 0,
    level: 12,
  },
];

const written = 1794304800;

function story(
  id: number,
  kind: Story["kind"],
  startDay: string,
  endDay: string,
  text: string,
  extra: Partial<Story> = {},
): Story {
  return {
    id,
    characterId: 1,
    kind,
    startDay,
    endDay,
    narrator: "murloc_scribe",
    language: "en",
    provider: "local",
    model: "qwen3.5-4b",
    promptHash: "demo",
    text,
    createdAt: written + id * 60,
    check: "passed",
    checkReason: null,
    status: "accepted",
    note: null,
    partial: false,
    ...extra,
  };
}

export const stories: Story[] = [
  story(
    1,
    "day",
    "2026-11-01",
    "2026-11-01",
    "The plains of Mulgore rolled out under a sky the colour of old parchment, and Wren walked them as if they owed her rent. Red Cloud Mesa came first, then the wide grass beyond it, both new to her hooves.\n\nThe water was less polite. She went in to look at something shiny and came out a ghost, drowned in a pond no deeper than a tauren is tall. She walked back to her body in silence, which for a tauren is saying something.\n\n* Mrgl. The pond keeps what the pond takes. The boot stays with us.",
  ),
  story(
    2,
    "day",
    "2026-11-08",
    "2026-11-08",
    "Red Rocks first, where the wind talks to the stones, and then the news that Thunder Bluff stood waiting on its mesas. Wren took the Ravaged Caravan to heart, found what the wolves had left, and carried the story back for 75 copper and a promotion: level 6, which she celebrated by standing very still for a portrait.\n\nHogger fell, a Carving Knife changed hands, and Tamsin joined her on the road. It was a good day right until the stream. A Murloc Streamrunner rose out of it with opinions, and the day ended with Wren face down in the reeds.\n\n* Mrglglgl! First one of us she met. Last thing she saw. Fair trade.",
  ),
  story(
    3,
    "day",
    "2026-11-09",
    "2026-11-09",
    "A quiet one. Wren sat on the pier at Bloodhoof Village with a line in the water and caught two Raw Slitherskin Mackerel, which is two more than the pond gave her last week. Nothing else happened, and she seemed to prefer it that way.",
    { narrator: "grumpy_dwarf" },
  ),
  story(
    4,
    "week",
    "2026-11-02",
    "2026-11-08",
    "A week that began on the road to Red Rocks and ended in a stream. In between, Wren learned where Thunder Bluff was, settled the matter of the Ravaged Caravan, reached level 6 and gained a companion in Tamsin.\n\nShe also met her first murloc, briefly and from below. The scribe has asked that this be recorded as a draw.",
  ),
];

export const narrators: NarratorInfo[] = [
  {
    id: "murloc_scribe",
    name: "The murloc scribe",
    optIn: false,
    custom: false,
  },
  { id: "grumpy_dwarf", name: "The grumpy dwarf", optIn: false, custom: false },
  {
    id: "pompous_noble",
    name: "The pompous noble",
    optIn: false,
    custom: false,
  },
  {
    id: "smug_academic",
    name: "The smug academic",
    optIn: false,
    custom: false,
  },
  {
    id: "easygoing_troll",
    name: "The easygoing troll",
    optIn: false,
    custom: false,
  },
  { id: "silly_elf", name: "The silly elf", optIn: false, custom: false },
  { id: "custom_heron", name: "The patient heron", optIn: false, custom: true },
];

export const heron: Narrator = {
  id: "custom_heron",
  language: "en",
  name: "The patient heron",
  opt_in: false,
  point_of_view: "adventurer",
  persona:
    "You are a grey heron who has watched this adventurer from the shallows for years.",
  voice: "Still, patient and fond of water. Short sentences. Never hurried.",
  murloc:
    "A murloc may splash at the edge of the page, but the heron does not look.",
  margin_note:
    "After the entry, add one footnote starting with *, written by the murloc in the margin.",
  example: {
    intro:
      "Here is an example of the notes you will get and an entry written from them.",
    notes_label: "Notes:",
    entry_label: "Entry:",
    notes:
      "Fished twice at Bloodhoof Village. Caught 2 Raw Slitherskin Mackerel.",
    entry:
      "Two fish today. She waited for each one the way I wait for mine. We understand each other.",
    margin_note: "* Mrgl. Heron eats fish. Suspicious.",
  },
  hints: { only_fished: "Linger on the waiting more than the catch." },
};

export const settings: SettingsView = {
  settings: {
    wowRootOverride: null,
    backupRetention: "days90",
    uiLanguage: null,
    site: {
      enabled: false,
      folder: null,
      themeFolder: null,
      title: "",
      baseUrl: null,
      characters: [],
      standInNames: true,
      hideRealm: true,
    },
    writing: {
      narrator: "murloc_scribe",
      meddling: "sometimes",
      proseLanguage: null,
      provider: "local",
      modelId: "qwen3.5-4b",
      endpointUrl: null,
      endpointModel: null,
      pseudonyms: false,
      writeWhilePlaying: false,
      writeOnBattery: false,
      paused: false,
      review: false,
      copyFooter: true,
      weekly: true,
      preferences: "No em dashes. Short paragraphs.",
    },
  },
  roots: [{ path: FOLDER, source: "defaultPath" }],
  unreadable: [],
};

export const writingStatus: WritingStatus = {
  paused: false,
  current: null,
  jobs: [],
  blocked: null,
};

export const models: ModelsView = {
  chosenId: "qwen3.5-4b",
  advice: { kind: "recommend", id: "qwen3.5-4b" },
  models: [
    {
      id: "qwen3.5-4b",
      name: "Qwen3.5 4B",
      sizeBytes: 2740937888,
      licence: "Apache-2.0",
      languages: ["en", "fr"],
      minRamGb: 8,
      recommended: true,
      proseNote: null,
      installed: true,
      partialBytes: 0,
    },
    {
      id: "ministral-3-3b",
      name: "Ministral 3B",
      sizeBytes: 2146497824,
      licence: "Apache-2.0",
      languages: ["en", "fr"],
      minRamGb: 8,
      recommended: false,
      proseNote: null,
      installed: false,
      partialBytes: 0,
    },
    {
      id: "smollm3-3b",
      name: "SmolLM3 3B",
      sizeBytes: 1915306528,
      licence: "Apache-2.0",
      languages: ["en", "fr"],
      minRamGb: 6,
      recommended: false,
      proseNote: null,
      installed: false,
      partialBytes: 0,
    },
  ],
};

export const addonViews: AddonView[] = [
  {
    flavorPath: `${FOLDER}/_forever_`,
    ownership: "managed",
    rollback: null,
    status: {
      flavor: "_forever_",
      folder: `${FOLDER}/_forever_/Interface/AddOns/Mrglglgl`,
      clientVersion: "1.60.1.70790",
      installed: {
        version: "0.1.0",
        interface: 16001,
        schema: 1,
        development: false,
      },
      lastRun: {
        character: "Wren",
        version: "0.1.0",
        interface: 16001,
        login: 1794304800,
      },
      state: "recording",
      problems: [],
    },
  },
];

export const releaseCheck: ReleaseCheck = {
  release: {
    tag: "addon-v0.1.0",
    version: "0.1.0",
    prerelease: false,
    url: "https://github.com/AshDevFr/mrglglgl/releases/download/addon-v0.1.0/Mrglglgl-addon-v0.1.0-forever.zip",
    size: 27687,
    sha256: "demo",
    published: "2026-11-06T22:00:00Z",
  },
  prereleases: false,
  testVersion: null,
  updates: [],
  checked: 1794304800,
  stale: false,
};
