import { useQuery } from "@tanstack/react-query";
import { listCharacters } from "@/api/commands";
import { displayName } from "@/lib/journal";
import { useCharacterNav } from "@/lib/useCharacterNav";
import { type CharacterTab, CharacterTabs } from "./CharacterTabs";

/** The character's name and its pages, at the top of its To review and Milestones pages. */
export function CharacterTitle({
  characterId,
  current,
}: {
  characterId: number;
  current: CharacterTab;
}) {
  const go = useCharacterNav(characterId);
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const summary = characters.data?.find((c) => c.character.id === characterId);
  return (
    <div style={{ maxWidth: "48rem", marginBottom: "1.5rem" }}>
      {summary && (
        <h1 className="character-name">{displayName(summary.character)}</h1>
      )}
      <CharacterTabs
        current={current}
        toReview={summary?.toReview ?? 0}
        onSelect={(tab) => void go(tab)}
      />
    </div>
  );
}
