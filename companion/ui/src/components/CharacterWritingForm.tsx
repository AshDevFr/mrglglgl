import { Button, Group, Select, Stack, Text, Textarea } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  characterWriting,
  getSettings,
  listNarrators,
  setCharacterWriting,
} from "@/api/commands";
import type { CharacterWriting } from "@/bindings/CharacterWriting";
import type { Meddling } from "@/bindings/Meddling";
import { narratorLabel } from "@/lib/narrators";

const MEDDLING: Meddling[] = ["off", "rare", "sometimes", "often"];
/** The choice that leaves a field to Settings. */
const AS_SETTINGS = "";
/** A character's own preferences; the backend keeps no more. */
const OWN_PREFERENCES_CHARS = 500;

/** How one character's entries are written where it differs from Settings: its narrator, how
 * often the murloc writes in the margin, and preferences added to the ones every character
 * shares. Each choice left to Settings says what Settings would use. */
export function CharacterWritingForm({ characterId }: { characterId: number }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const own = useQuery({
    queryKey: ["characterWriting", characterId],
    queryFn: () => characterWriting(characterId),
  });
  const settings = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
  });
  const [draft, setDraft] = useState<CharacterWriting | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (own.data) setDraft(own.data);
  }, [own.data]);

  const general = settings.data?.settings.writing;
  if (!draft || !general || !narrators.data) return null;

  const set = (change: Partial<CharacterWriting>) => {
    setSaved(false);
    setDraft({ ...draft, ...change });
  };
  async function save() {
    if (!draft) return;
    await setCharacterWriting(characterId, draft);
    await queryClient.invalidateQueries({
      queryKey: ["characterWriting", characterId],
    });
    setSaved(true);
  }

  const asSettings = (what: string) =>
    t("characterSettings.asSettings", { what });
  return (
    <Stack gap="md">
      <Text size="sm" c="dimmed">
        {t("characterSettings.intro")}
      </Text>
      <Select
        label={t("settings.narrator")}
        data={[
          {
            value: AS_SETTINGS,
            label: asSettings(
              narratorLabel(t, narrators.data, general.narrator),
            ),
          },
          ...narrators.data.map((n) => ({ value: n.id, label: n.name })),
        ]}
        value={draft.narrator ?? AS_SETTINGS}
        onChange={(v) => set({ narrator: v ? v : null })}
        allowDeselect={false}
      />
      <Select
        label={t("settings.meddling")}
        data={[
          {
            value: AS_SETTINGS,
            label: asSettings(t(`settings.meddlingLevel.${general.meddling}`)),
          },
          ...MEDDLING.map((m) => ({
            value: m,
            label: t(`settings.meddlingLevel.${m}`),
          })),
        ]}
        value={draft.meddling ?? AS_SETTINGS}
        onChange={(v) => set({ meddling: v ? (v as Meddling) : null })}
        allowDeselect={false}
      />
      <Textarea
        label={t("characterSettings.preferences")}
        description={
          general.preferences.trim()
            ? t("characterSettings.preferencesAdded", {
                general: general.preferences.trim(),
              })
            : t("characterSettings.preferencesAlone")
        }
        placeholder={t("characterSettings.preferencesPlaceholder")}
        value={draft.preferences}
        maxLength={OWN_PREFERENCES_CHARS}
        onChange={(e) => set({ preferences: e.currentTarget.value })}
        autosize
        minRows={4}
        maxRows={12}
      />
      <Group gap="sm">
        <Button onClick={() => void save()}>
          {t("characterSettings.save")}
        </Button>
        {saved && (
          <Text size="sm" c="dimmed" role="status">
            {t("characterSettings.saved")}
          </Text>
        )}
      </Group>
    </Stack>
  );
}
