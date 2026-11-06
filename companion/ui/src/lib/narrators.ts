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

/** The editor's address for a narrator not made yet; saved ones have `custom_` ids. */
export const NEW_NARRATOR = "new";
