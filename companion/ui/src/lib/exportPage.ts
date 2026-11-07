import i18n from "i18next";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  characterJournalShared,
  characterMilestonesShared,
  characterScreenshots,
  characterStories,
  listCharacters,
  screenshotData,
  screenshotUrl,
} from "@/api/commands";
import {
  type ExportData,
  ExportDocument,
  type ExportPhoto,
} from "@/components/ExportDocument";
import { displayName } from "@/lib/journal";
import { savedPaper } from "@/lib/paper";
import exportCss from "@/styles/export.css?raw";
import paperCss from "@/styles/paper.css?raw";

/** Sets the page's theme before it is drawn: the reader's own choice if they made one here,
 * else their system's light or dark, on the paper the journal was exported on. The buttons
 * switch either; printing is always on light paper. Inline, so the file needs nothing else. */
function themeScript(paper: string): string {
  return `(function () {
  var root = document.documentElement, key = "mrglglgl.export.";
  var attr = { scheme: "data-mantine-color-scheme", paper: "data-paper" };
  function saved(name) { try { return localStorage.getItem(key + name); } catch (e) { return null; } }
  function show(name, value, keep) {
    root.setAttribute(attr[name], value);
    if (keep) { try { localStorage.setItem(key + name, value); } catch (e) {} }
  }
  var dark = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  show("scheme", saved("scheme") || (dark ? "dark" : "light"));
  show("paper", saved("paper") || ${JSON.stringify(paper)});
  document.addEventListener("click", function (event) {
    var button = event.target.closest && event.target.closest("[data-toggle]");
    if (!button) return;
    var name = button.getAttribute("data-toggle");
    var now = root.getAttribute(attr[name]);
    show(name, name === "scheme" ? (now === "dark" ? "light" : "dark")
      : (now === "parchment" ? "tide" : "parchment"), true);
  });
  var before;
  window.addEventListener("beforeprint", function () {
    before = root.getAttribute(attr.scheme);
    root.setAttribute(attr.scheme, "light");
  });
  window.addEventListener("afterprint", function () {
    if (before) root.setAttribute(attr.scheme, before);
  });
})();`;
}

/** The journal as one web page's body and look, from the components the app shows, with
 * controls for light or dark and the paper. The core adds the font and writes the file. */
export function exportPage(data: ExportData): { css: string; body: string } {
  const t = i18n.t.bind(i18n);
  const controls =
    '<div class="export-controls no-print">' +
    `<button type="button" data-toggle="scheme">${t("export.toggleScheme")}</button>` +
    `<button type="button" data-toggle="paper">${t("export.togglePaper")}</button>` +
    "</div>";
  return {
    css: `${paperCss}\n${exportCss}`,
    body: `<script>${themeScript(data.paper)}</script>\n${controls}\n${renderToStaticMarkup(
      createElement(ExportDocument, data),
    )}`,
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
    characterJournalShared(characterId),
    characterStories(characterId),
    characterMilestonesShared(characterId),
    characterScreenshots(characterId),
  ]);
  if (!journal) throw new Error("this character has no journal yet");
  const summary = characters.find((c) => c.character.id === characterId);
  const photos: Record<string, ExportPhoto[]> = {};
  for (const [day, list] of Object.entries(shots)) {
    photos[day] = await Promise.all(
      list.map(async (s) => ({
        src: embed
          ? await screenshotData(characterId, s.path)
          : screenshotUrl(s.path),
        moment: s.moment,
        taken: s.taken,
      })),
    );
  }
  return {
    title: summary ? displayName(summary.character) : "Journal",
    journal,
    stories,
    awards,
    photos,
    paper: savedPaper(),
  };
}
