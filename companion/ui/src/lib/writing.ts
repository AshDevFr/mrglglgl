import type { Job } from "@/bindings/Job";
import type { QueueEvent } from "@/bindings/QueueEvent";
import type { Story } from "@/bindings/Story";
import type { StoryKind } from "@/bindings/StoryKind";

/** What the window knows of an entry being written, or just finished. */
export interface Live {
  /** The text streamed so far. */
  text: string;
  active: boolean;
  /** How the last attempt ended: "written", "setAside", "cancelled", "timedOut", "failed". */
  outcome: string | null;
  message: string | null;
  /** The narrator writing it, from the moment it starts. */
  narrator: string | null;
}

export type LiveMap = Record<string, Live>;

export const slot = (kind: StoryKind, day: string) => `${kind}:${day}`;

/** The id of a day or week's card in To review, for a link straight to it. */
export const reviewAnchor = (kind: StoryKind, day: string) =>
  `review-${kind}-${day}`;

const idle: Live = {
  text: "",
  active: false,
  outcome: null,
  message: null,
  narrator: null,
};

/** Folds one writer event into what the window shows for a character. */
export function applyEvent(
  map: LiveMap,
  characterId: number,
  event: QueueEvent,
): LiveMap {
  if (event.characterId !== characterId) return map;
  const key = slot(event.kind, event.startDay);
  const current = map[key] ?? idle;
  switch (event.type) {
    case "started":
      return {
        ...map,
        [key]: { ...idle, active: true, narrator: event.narrator },
      };
    case "restarted":
      return {
        ...map,
        [key]: { ...idle, active: true, narrator: current.narrator },
      };
    case "chunk":
      return { ...map, [key]: { ...current, text: current.text + event.text } };
    case "finished":
      return {
        ...map,
        [key]: {
          text: "",
          active: false,
          outcome: event.outcome,
          message: event.message,
          narrator: current.narrator,
        },
      };
  }
}

/** Versions of one day or week, newest first. */
export function versionsOf(
  stories: Story[],
  kind: StoryKind,
  day: string,
): Story[] {
  return stories
    .filter((s) => s.kind === kind && s.startDay === day)
    .sort((a, b) => b.id - a.id);
}

export function jobFor(
  jobs: Job[],
  kind: StoryKind,
  day: string,
): Job | undefined {
  return jobs.find((j) => j.kind === kind && j.startDay === day);
}

const LOADING = [
  "writing.loading.writing",
  "writing.loading.drying",
  "writing.loading.translating",
];

/** One of the loading lines, the same one for the same day. */
export function loadingKey(day: string): string {
  let sum = 0;
  for (const c of day) sum += c.charCodeAt(0);
  return LOADING[sum % LOADING.length];
}
