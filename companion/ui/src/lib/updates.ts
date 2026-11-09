import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { describe, log } from "./log";

/** Looks for an update, logging what it found or why it could not look. */
async function checked(asked: boolean): Promise<Update | null> {
  log("info", `looking for an app update${asked ? ", asked for" : ""}`);
  try {
    const update = await check();
    log(
      "info",
      update
        ? `app update found: ${update.currentVersion} -> ${update.version}`
        : "no app update: this is the newest version",
    );
    return update;
  } catch (error) {
    log("error", `the update check failed: ${describe(error)}`);
    throw error;
  }
}

/** A newer version published on GitHub, or null. Prereleases are never offered: the app reads
 * the latest stable release's update file. A failed check (offline, no release yet, an update
 * file not published) is not the player's problem and reads as none. */
export async function availableUpdate(): Promise<Update | null> {
  if (import.meta.env.VITE_DEMO === "1") return null;
  try {
    return await checked(false);
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
    return { update: await checked(true), failed: false };
  } catch {
    return { update: null, failed: true };
  }
}

/** Downloads and installs the update, then restarts into it. */
export async function installAndRestart(update: Update): Promise<void> {
  log("info", `installing app update ${update.version}`);
  let downloaded = 0;
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === "Started")
        log(
          "debug",
          `update download started: ${event.data.contentLength ?? "?"} bytes`,
        );
      else if (event.event === "Progress") downloaded += event.data.chunkLength;
      else log("info", `update downloaded (${downloaded} bytes), installing`);
    });
  } catch (error) {
    log("error", `the update did not install: ${describe(error)}`);
    throw error;
  }
  log("info", "update installed, restarting");
  await relaunch();
}
