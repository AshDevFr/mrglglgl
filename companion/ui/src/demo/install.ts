// Demo mode: the app's own interface in a plain browser, with every backend command answered
// from made-up data. Used to take the documentation's screenshots in CI, where the desktop app
// cannot run. Built only with `VITE_DEMO=1`; the shipped app never includes it.
import {
  mockConvertFileSrc,
  mockIPC,
  mockWindows,
} from "@tauri-apps/api/mocks";
import packageInfo from "../../package.json";
import * as data from "./data";

type Args = Record<string, unknown>;

const answers: Record<string, (args: Args) => unknown> = {
  app_info: () => ({ version: packageInfo.version }),
  // "?empty" shows the app as on a first run, before any journal arrives.
  list_characters: () =>
    new URLSearchParams(window.location.search).has("empty")
      ? []
      : data.characters,
  character_journal: () => data.journal,
  character_milestones: () => data.awards,
  character_journal_shared: () => data.journal,
  character_milestones_shared: () => data.awards,
  character_data_use: () => ({ shared: [], view: [] }),
  set_character_data_use: () => null,
  set_character_enabled: () => null,
  character_screenshots: () => ({}),
  character_stories: () => data.stories,
  // A few lines of each level, so the Logs tab shows its colours.
  recent_log: () => [
    "2026-11-10T02:00:00Z  INFO mrglglgl::sync: sync: 1 characters, 1 imported, 0 unreadable, in 41ms",
    "2026-11-10T02:00:00Z DEBUG mrglglgl::sync: imported Wren-Hollowbrook (character 1) in 38ms",
    "2026-11-10T02:00:01Z TRACE mrglglgl_core::queue: character 1: nothing new to write",
    "2026-11-10T02:01:12Z  INFO mrglglgl_core::queue: character 1: day of 2026-11-08 written in 41.2s",
    "2026-11-10T02:03:40Z  WARN mrglglgl_core::writer: entry asked again, it got facts wrong: says 17 wolves, the notes say 23",
    "2026-11-10T02:05:02Z ERROR mrglglgl_llm::endpoint: the endpoint refused: generation failed: the endpoint answered 500",
    "  caused by: backend model is not loaded",
  ],
  review_list: () => [],
  writing_status: () => data.writingStatus,
  get_settings: () => data.settings,
  save_settings: () => ({
    roots: data.settings.roots,
    characters: 1,
    imported: 0,
    unreadable: 0,
  }),
  sync_now: () => ({
    roots: data.settings.roots,
    characters: 1,
    imported: 0,
    unreadable: 0,
  }),
  list_narrators: () => data.narrators,
  narrator: (args) => (args.id === data.heron.id ? data.heron : null),
  blank_narrator: () => ({
    ...data.heron,
    id: "",
    name: "",
    persona: "",
    voice: "",
    example: { ...data.heron.example, entry: "", margin_note: "" },
    hints: {},
  }),
  copy_narrator: () => ({ ...data.heron, id: "" }),
  models: () => data.models,
  has_api_key: () => false,
  endpoint_sends_off_machine: () => false,
  addon_views: () => data.addonViews,
  addon_release: () => data.releaseCheck,
  default_export_folder: () => "/Users/wren/Documents",
  site_stand_in_names: () => [],
  copy_entry_text: () => "",
};

/** Commands that change something answer as if they succeeded and change nothing. */
function answer(cmd: string, args: Args): unknown {
  const known = answers[cmd];
  if (known) return known(args);
  if (cmd.startsWith("plugin:event|")) return 0;
  return null;
}

mockWindows("main");
mockConvertFileSrc("macos");
mockIPC((cmd, args) => answer(cmd, (args ?? {}) as Args), {
  shouldMockEvents: true,
});
