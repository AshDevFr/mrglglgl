import { Alert, Checkbox, Stack, Table, Text, Title } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  getSettings,
  listCharacters,
  saveSiteSettings,
  setCharacterEnabled,
} from "@/api/commands";
import type { CharacterSummary } from "@/bindings/CharacterSummary";
import { problem } from "@/i18n/problems";
import { characterLine, displayName } from "@/lib/journal";
import { scheduleRebuild } from "@/lib/siteBuild";

/** Every character the app has found, each in the app or not, and on the website or not. */
export function CharactersPanel() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const settings = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const onSite = new Set(settings.data?.settings.site.characters ?? []);
  const [error, setError] = useState<string | null>(null);

  // The website's list is read as stored when it changes, so an edit elsewhere is kept.
  async function setOnSite(id: number, on: boolean) {
    const site = (await getSettings()).settings.site;
    const others = site.characters.filter((c) => c !== id);
    await saveSiteSettings({
      ...site,
      characters: on ? [...others, id] : others,
    });
    scheduleRebuild();
  }

  async function change(run: () => Promise<unknown>) {
    setError(null);
    try {
      await run();
    } catch (e) {
      setError(String(e));
    }
    await queryClient.invalidateQueries();
  }

  const row = ({ character, level }: CharacterSummary) => {
    const name = displayName(character);
    const line = characterLine(t, character, level);
    return (
      <Table.Tr key={character.id} data-out={!character.enabled || undefined}>
        <Table.Td>
          <Text c={character.enabled ? undefined : "dimmed"}>{name}</Text>
          <Text size="xs" c="dimmed">
            {[line, character.realm ?? character.realmFolder, character.account]
              .filter(Boolean)
              .join(" · ")}
          </Text>
        </Table.Td>
        <Table.Td>
          <Checkbox
            aria-label={`${name}: ${t("characters.inApp")}`}
            checked={character.enabled}
            onChange={(e) => {
              const on = e.currentTarget.checked;
              void change(async () => {
                await setCharacterEnabled(character.id, on);
                // Out of the app is off the site too: nothing of it is read or written.
                if (!on && onSite.has(character.id))
                  await setOnSite(character.id, false);
              });
            }}
          />
        </Table.Td>
        <Table.Td>
          <Checkbox
            aria-label={`${name}: ${t("characters.onSite")}`}
            checked={character.enabled && onSite.has(character.id)}
            disabled={!character.enabled}
            onChange={(e) => {
              const on = e.currentTarget.checked;
              void change(() => setOnSite(character.id, on));
            }}
          />
        </Table.Td>
      </Table.Tr>
    );
  };

  return (
    <Stack>
      <Title order={2} size="h3">
        {t("characters.title")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("characters.help")}
      </Text>
      {error && (
        <Alert color="red">
          {problem(t, "settings.error", { message: error })}
        </Alert>
      )}
      {characters.data && characters.data.length === 0 ? (
        <Text size="sm">{t("characters.none")}</Text>
      ) : (
        <Table>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>{t("characters.character")}</Table.Th>
              <Table.Th>{t("characters.inApp")}</Table.Th>
              <Table.Th>{t("characters.onSite")}</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>{(characters.data ?? []).map(row)}</Table.Tbody>
        </Table>
      )}
    </Stack>
  );
}
