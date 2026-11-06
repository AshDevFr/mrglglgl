import { useNavigate } from "@tanstack/react-router";
import type { CharacterTab } from "@/components/CharacterTabs";
import { lastDay } from "@/lib/lastDay";

/** Goes to one of a character's pages; the journal reopens on the day last open. */
export function useCharacterNav(characterId: number) {
  const navigate = useNavigate();
  const params = { characterId: String(characterId) };
  return (tab: CharacterTab) => {
    if (tab === "journal") {
      const day = lastDay(characterId);
      return navigate({
        to: "/characters/$characterId",
        params,
        search: day ? { day } : {},
      });
    }
    return navigate({
      to:
        tab === "review"
          ? "/characters/$characterId/review"
          : "/characters/$characterId/milestones",
      params,
    });
  };
}
