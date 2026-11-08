import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

/** A newer version published on GitHub, or null. Prereleases are never offered: the app reads
 * the latest stable release's update file. A failed check (offline, no release yet, an update
 * file not published) is not the player's problem and reads as none. */
export async function availableUpdate(): Promise<Update | null> {
  if (import.meta.env.VITE_DEMO === "1") return null;
  try {
    return await check();
  } catch {
    return null;
  }
}

/** The same check, asked for by the player, who needs to know when it failed: offline, or
 * GitHub not answering. */
export async function checkForUpdate(): Promise<{
  update: Update | null;
  failed: boolean;
}> {
  if (import.meta.env.VITE_DEMO === "1") return { update: null, failed: false };
  try {
    return { update: await check(), failed: false };
  } catch {
    return { update: null, failed: true };
  }
}

/** Downloads and installs the update, then restarts into it. */
export async function installAndRestart(update: Update): Promise<void> {
  await update.downloadAndInstall();
  await relaunch();
}
