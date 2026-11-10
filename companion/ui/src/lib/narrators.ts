import type { TFunction } from "i18next";
import type { NarratorInfo } from "@/bindings/NarratorInfo";

/** A narrator's name for a byline. One the player removed is named as such, not by its id. */
export function narratorLabel(
  t: TFunction,
  narrators: NarratorInfo[],
  id: string,
): string {
  const found = narrators.find((n) => n.id === id);
  if (found) return found.name;
  return id.startsWith("custom_") ? t("narrators.removed") : id;
}

/** The choices of a narrator picker, the character's default one marked as such, since an
 * entry with none picked is written by it. */
export function narratorChoices(
  t: TFunction,
  narrators: NarratorInfo[],
  defaultId?: string,
): { value: string; label: string }[] {
  return narrators.map((n) => ({
    value: n.id,
    label:
      n.id === defaultId ? t("narrators.isDefault", { name: n.name }) : n.name,
  }));
}

/** A picker's placeholder when nothing is picked: who writes then. */
export function defaultPlaceholder(
  t: TFunction,
  narrators: NarratorInfo[],
  defaultId: string | undefined,
  fallback: string,
): string {
  return defaultId
    ? t("narrators.defaultIs", { name: narratorLabel(t, narrators, defaultId) })
    : fallback;
}

/** The editor's address for a narrator not made yet; saved ones have `custom_` ids. */
export const NEW_NARRATOR = "new";
