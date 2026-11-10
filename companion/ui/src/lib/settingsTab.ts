// Which Settings tab opens: the one last open in this session, or one another page asks for
// (the home page's "Install the addon" opens the Addon tab).
export const SETTINGS_TABS = [
  "general",
  "writing",
  "characters",
  "narrators",
  "website",
  "addon",
  "logs",
] as const;
export type SettingsTab = (typeof SETTINGS_TABS)[number];
const KEY = "mrglglgl.settingsTab";

export function rememberedSettingsTab(): SettingsTab {
  try {
    const saved = sessionStorage.getItem(KEY);
    return SETTINGS_TABS.find((tab) => tab === saved) ?? "general";
  } catch {
    return "general";
  }
}

export function rememberSettingsTab(tab: SettingsTab) {
  try {
    sessionStorage.setItem(KEY, tab);
  } catch {
    // Not remembered; Settings opens on its first tab.
  }
}
