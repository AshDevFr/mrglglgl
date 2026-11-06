import { Stack } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { Journal } from "@/bindings/Journal";
import type { StoredCharacter } from "@/bindings/StoredCharacter";
import { formatTime } from "@/lib/format";
import { displayName, latestSnapshot } from "@/lib/journal";

interface Props {
  character: StoredCharacter;
  journal: Journal | null;
}

/** Name and surname, then level, where they were last seen, boots kept and last synced. */
export function CharacterHeader({ character, journal }: Props) {
  const { t, i18n } = useTranslation();
  const snapshot = journal
    ? latestSnapshot(journal)
    : { level: null, zone: null };
  return (
    <Stack gap={6}>
      <h1 className="character-name">{displayName(character)}</h1>
      <div className="character-meta">
        {snapshot.level !== null && (
          <span>{t("character.level", { level: snapshot.level })}</span>
        )}
        {snapshot.zone && <span>{snapshot.zone}</span>}
        {journal && (
          <span>{t("character.boots", { count: journal.stats.boots })}</span>
        )}
        <span>
          {character.lastSynced === null
            ? t("sync.neverSynced")
            : t("sync.lastSynced", {
                when: formatTime(character.lastSynced, i18n.language),
              })}
        </span>
      </div>
    </Stack>
  );
}
