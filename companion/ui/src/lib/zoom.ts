import { getCurrentWebview } from "@tauri-apps/api/webview";

/** The interface sizes offered, in percent. */
export const ZOOMS = [80, 90, 100, 110, 125, 150, 175];

const KEY = "mrglglgl.zoom";
/** Sent on the window when the size changes, so the Settings control follows the shortcuts. */
export const ZOOM_CHANGED = "mrglglgl-zoom";

/** The size the player chose on this computer, 100% unless one of the offered sizes. It lives in
 * the window's own storage: a display preference, not journal data. */
export function savedZoom(): number {
  try {
    const value = Number(localStorage.getItem(KEY));
    return ZOOMS.includes(value) ? value : 100;
  } catch {
    return 100;
  }
}

/** The next offered size up (1) or down (-1), or 100% for 0. */
export function stepZoom(current: number, direction: 1 | -1 | 0): number {
  if (direction === 0) return 100;
  const at = ZOOMS.indexOf(current);
  const index = at === -1 ? ZOOMS.indexOf(100) : at;
  return ZOOMS[Math.min(ZOOMS.length - 1, Math.max(0, index + direction))];
}

/** Sizes the whole window, text and layout together, and remembers the choice. */
export async function applyZoom(percent: number): Promise<void> {
  try {
    localStorage.setItem(KEY, String(percent));
  } catch {
    // The size still applies for this session.
  }
  window.dispatchEvent(new CustomEvent(ZOOM_CHANGED, { detail: percent }));
  try {
    await getCurrentWebview().setZoom(percent / 100);
  } catch {
    // Outside the app window (tests, a browser) there is nothing to zoom.
  }
}
