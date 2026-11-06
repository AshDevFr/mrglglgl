import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  characterJournal,
  characterMilestones,
  characterScreenshots,
  characterStories,
  listCharacters,
  screenshotData,
  screenshotUrl,
} from "@/api/commands";
import { type ExportData, ExportDocument } from "@/components/ExportDocument";
import { displayName } from "@/lib/journal";
import exportCss from "@/styles/export.css?raw";
import paperCss from "@/styles/paper.css?raw";

/** The journal as one web page's body and look, from the components the app shows. The core
 * adds the font and writes the file. */
export function exportPage(data: ExportData): { css: string; body: string } {
  return {
    css: `${paperCss}\n${exportCss}`,
    body: renderToStaticMarkup(createElement(ExportDocument, data)),
  };
}

/** Everything an export tells, from the app's own records. Pictures are embedded as data for a
 * file (`embed`), or shown from the screenshot folder for printing. */
export async function loadExportData(
  characterId: number,
  embed: boolean,
): Promise<ExportData> {
  const [characters, journal, stories, awards, shots] = await Promise.all([
    listCharacters(),
    characterJournal(characterId),
    characterStories(characterId),
    characterMilestones(characterId),
    characterScreenshots(characterId),
  ]);
  if (!journal) throw new Error("this character has no journal yet");
  const summary = characters.find((c) => c.character.id === characterId);
  const photos: Record<string, string[]> = {};
  for (const [day, list] of Object.entries(shots)) {
    photos[day] = await Promise.all(
      list.map((s) =>
        embed ? screenshotData(characterId, s.path) : screenshotUrl(s.path),
      ),
    );
  }
  return {
    title: summary ? displayName(summary.character) : "Journal",
    journal,
    stories,
    awards,
    photos,
  };
}
