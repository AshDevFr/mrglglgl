import { defaultExportFolder } from "@/api/commands";

const KEY = "mrglglgl.exportFolder";

/** The folder exports open on: the last one used on this computer, else Documents. */
export async function exportFolder(): Promise<string> {
  try {
    const last = localStorage.getItem(KEY);
    if (last) return last;
  } catch {
    // Fall back to the default folder.
  }
  return (await defaultExportFolder()) ?? "";
}

/** Remembers the folder of an export that was saved. */
export function rememberExportFolder(folder: string): void {
  try {
    localStorage.setItem(KEY, folder.trim());
  } catch {
    // The next export simply opens on the default folder.
  }
}
