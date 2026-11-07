import { Button, Checkbox, Group, Modal, Stack, Text } from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { characterUnusedData, setCharacterUnusedData } from "@/api/commands";

/** The recorded categories a player can set aside per character, as the addon records them. */
export const DATA_CATEGORIES = [
  "sessions",
  "levels",
  "quests",
  "zones",
  "kills",
  "deaths",
  "fishing",
  "professions",
  "loot",
  "money",
  "group",
  "chat",
] as const;

/** Which recorded data the app uses for one character. Everything is on until the player turns
 * something off; off hides it in the window, from narrators, the website and exports, and the
 * archive keeps it so turning it back on brings it back. */
export function DataUseDialog({
  characterId,
  opened,
  onClose,
}: {
  characterId: number;
  opened: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const unused = useQuery({
    queryKey: ["unusedData", characterId],
    queryFn: () => characterUnusedData(characterId),
    enabled: opened,
  });
  const [off, setOff] = useState<Set<string>>(new Set());
  useEffect(() => {
    if (unused.data) setOff(new Set(unused.data));
  }, [unused.data]);
  const save = useMutation({
    mutationFn: () => setCharacterUnusedData(characterId, [...off]),
    onSuccess: async () => {
      // Days, milestones and counts all change with what is in use.
      await queryClient.invalidateQueries();
      onClose();
    },
  });

  return (
    <Modal opened={opened} onClose={onClose} title={t("dataUse.title")}>
      <Stack>
        <Text size="sm">{t("dataUse.body")}</Text>
        {DATA_CATEGORIES.map((category) => (
          <Checkbox
            key={category}
            label={t(`dataUse.category.${category}`)}
            checked={!off.has(category)}
            onChange={(e) => {
              const next = new Set(off);
              if (e.currentTarget.checked) next.delete(category);
              else next.add(category);
              setOff(next);
            }}
          />
        ))}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {t("character.cancel")}
          </Button>
          <Button loading={save.isPending} onClick={() => save.mutate()}>
            {t("dataUse.save")}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
