import {
  Button,
  Checkbox,
  Group,
  Modal,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { characterDataUse, setCharacterDataUse } from "@/api/commands";

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

type Audience = "shared" | "view";
const AUDIENCES: Audience[] = ["shared", "view"];

/** Which recorded data the app uses for one character, twice over: for narrators, the website
 * and exports, and for the window's "What happened". Everything is on until the player turns
 * something off; the archive keeps it all, so turning it back on brings it back. */
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
  const stored = useQuery({
    queryKey: ["dataUse", characterId],
    queryFn: () => characterDataUse(characterId),
    enabled: opened,
  });
  const [off, setOff] = useState<Record<Audience, Set<string>>>({
    shared: new Set(),
    view: new Set(),
  });
  useEffect(() => {
    if (stored.data)
      setOff({
        shared: new Set(stored.data.shared),
        view: new Set(stored.data.view),
      });
  }, [stored.data]);
  const save = useMutation({
    mutationFn: () =>
      setCharacterDataUse(characterId, {
        shared: [...off.shared],
        view: [...off.view],
      }),
    onSuccess: async () => {
      // Days, milestones and counts all change with what is in use.
      await queryClient.invalidateQueries();
      onClose();
    },
  });
  const toggle = (audience: Audience, category: string, on: boolean) => {
    const next = new Set(off[audience]);
    if (on) next.delete(category);
    else next.add(category);
    setOff({ ...off, [audience]: next });
  };

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={t("dataUse.title")}
      size="lg"
    >
      <Stack>
        <Text size="sm">{t("dataUse.body")}</Text>
        <Table>
          <Table.Thead>
            <Table.Tr>
              <Table.Th />
              {AUDIENCES.map((audience) => (
                <Table.Th key={audience}>{t(`dataUse.${audience}`)}</Table.Th>
              ))}
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {DATA_CATEGORIES.map((category) => {
              const label = t(`dataUse.category.${category}`);
              return (
                <Table.Tr key={category}>
                  <Table.Td>{label}</Table.Td>
                  {AUDIENCES.map((audience) => (
                    <Table.Td key={audience}>
                      <Checkbox
                        aria-label={`${label}: ${t(`dataUse.${audience}`)}`}
                        checked={!off[audience].has(category)}
                        onChange={(e) =>
                          toggle(audience, category, e.currentTarget.checked)
                        }
                      />
                    </Table.Td>
                  ))}
                </Table.Tr>
              );
            })}
          </Table.Tbody>
        </Table>
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
