import { Badge, Group, NavLink, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { CharacterSummary } from "@/bindings/CharacterSummary";
import { formatTime } from "@/lib/format";
import { displayName, groupCharacters } from "@/lib/journal";

interface Props {
  characters: CharacterSummary[];
  selectedId: number | null;
  onSelect: (characterId: number) => void;
}

/** Accounts, then realms, then characters, each with when it was last synced. */
export function CharacterTree({ characters, selectedId, onSelect }: Props) {
  const { t, i18n } = useTranslation();
  return (
    <Stack gap="xs">
      {groupCharacters(characters).map((account) => (
        <div key={account.account}>
          <Text size="sm" c="dimmed" fw={600}>
            {account.account}
          </Text>
          {account.realms.map((realm) => (
            <NavLink
              component="button"
              key={realm.realm}
              label={realm.realm}
              defaultOpened
            >
              {realm.characters.map(({ character, archived }) => (
                <NavLink
                  component="button"
                  key={character.id}
                  active={character.id === selectedId}
                  onClick={() => onSelect(character.id)}
                  label={
                    <Group gap={6}>
                      <span>{displayName(character)}</span>
                      {archived && (
                        <Badge
                          size="xs"
                          variant="light"
                          title={t("sync.archivedHelp")}
                        >
                          {t("sync.archived")}
                        </Badge>
                      )}
                    </Group>
                  }
                  description={
                    character.lastSynced === null
                      ? t("sync.neverSynced")
                      : t("sync.lastSynced", {
                          when: formatTime(character.lastSynced, i18n.language),
                        })
                  }
                />
              ))}
            </NavLink>
          ))}
        </div>
      ))}
    </Stack>
  );
}
