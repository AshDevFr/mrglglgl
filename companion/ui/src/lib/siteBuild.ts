import { useSyncExternalStore } from "react";
import { buildSite, getSettings } from "@/api/commands";
import type { SiteSettings } from "@/bindings/SiteSettings";
import type { WriteReport } from "@/bindings/WriteReport";
import { buildSiteData } from "@/lib/siteData";

/** What the last build of the website did. */
export interface SiteBuild {
  /** Unix time, in milliseconds. */
  at: number;
  report: WriteReport | null;
  error: string | null;
}

/** How long after the last change the site is rebuilt: a burst of changes builds once. */
export const REBUILD_DELAY_MS = 5000;

let last: SiteBuild | null = null;
let building = false;
const listeners = new Set<() => void>();
const notify = () => {
  for (const listener of listeners) listener();
};
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};
let timer: ReturnType<typeof setTimeout> | null = null;

export function lastBuild(): SiteBuild | null {
  return last;
}

/** The last build, kept up to date in a component. */
export function useLastBuild(): SiteBuild | null {
  return useSyncExternalStore(subscribe, lastBuild);
}

/** Whether the website is being built right now, kept up to date in a component. */
export function useSiteBuilding(): boolean {
  return useSyncExternalStore(subscribe, () => building);
}

/** Whether the site is set up enough to build. */
export function canBuild(site: SiteSettings): boolean {
  return site.folder !== null && site.characters.length > 0;
}

/** Builds the site now and records the result. */
export async function rebuildSite(site: SiteSettings): Promise<SiteBuild> {
  let result: SiteBuild;
  building = true;
  notify();
  try {
    const report = await buildSite(await buildSiteData(site));
    result = { at: Date.now(), report, error: null };
  } catch (e) {
    result = { at: Date.now(), report: null, error: String(e) };
  }
  last = result;
  building = false;
  notify();
  return result;
}

/** Rebuilds the site a few seconds after the last call, when it is on and set up. */
export function scheduleRebuild(): void {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    timer = null;
    void getSettings().then((view) => {
      const site = view.settings.site;
      if (site.enabled && canBuild(site)) void rebuildSite(site);
    });
  }, REBUILD_DELAY_MS);
}
