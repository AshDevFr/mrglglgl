import {
  ActionIcon,
  Button,
  Group,
  Menu,
  Modal,
  Stack,
  Text,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  characterJournal,
  deleteCharacter,
  listCharacters,
  setWritingPaused,
  writingStatus,
} from "@/api/commands";
import { displayName } from "@/lib/journal";
import { useCharacterNav } from "@/lib/useCharacterNav";
import { CharacterHeader } from "./CharacterHeader";
import { type CharacterTab, CharacterTabs } from "./CharacterTabs";
import { ExportDialog } from "./Sharing";

/** The top of every character page, the same on each tab: name, level, place, boots and last
 * synced, the Export and more menus, and the tabs. */
export function CharacterTop({
  characterId,
  current,
}: {
  characterId: number;
  current: CharacterTab;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const go = useCharacterNav(characterId);
  const [confirming, setConfirming] = useState(false);
  const [exporting, setExporting] = useState(false);

  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const journal = useQuery({
    queryKey: ["journal", characterId],
    queryFn: () => characterJournal(characterId),
  });
  const writingState = useQuery({
    queryKey: ["writing", characterId],
    queryFn: () => writingStatus(characterId),
  });
  const remove = useMutation({
    mutationFn: () => deleteCharacter(characterId),
    onSuccess: async () => {
      setConfirming(false);
      await queryClient.invalidateQueries();
      await navigate({ to: "/" });
    },
  });

  const summary = characters.data?.find((c) => c.character.id === characterId);
  if (!summary) return null;
  const name = displayName(summary.character);

  return (
    <>
      <Group justify="space-between" align="flex-start" wrap="wrap">
        <CharacterHeader
          character={summary.character}
          journal={journal.data ?? null}
        />
        <Group gap="xs" wrap="nowrap">
          <Button variant="default" onClick={() => setExporting(true)}>
            {t("share.exportButton")}
          </Button>
          <Menu position="bottom-end" withinPortal>
            <Menu.Target>
              <ActionIcon
                variant="default"
                size="lg"
                aria-label={t("nav.more")}
              >
                ⋯
              </ActionIcon>
            </Menu.Target>
            <Menu.Dropdown>
              {writingState.data && (
                <Menu.Item
                  onClick={() =>
                    void setWritingPaused(!writingState.data.paused).then(() =>
                      queryClient.invalidateQueries({
                        queryKey: ["writing", characterId],
                      }),
                    )
                  }
                >
                  {writingState.data.paused
                    ? t("writing.resume")
                    : t("writing.pause")}
                </Menu.Item>
              )}
              <Menu.Divider />
              <Menu.Item color="red" onClick={() => setConfirming(true)}>
                {t("character.remove")}
              </Menu.Item>
            </Menu.Dropdown>
          </Menu>
        </Group>
      </Group>
      <CharacterTabs
        current={current}
        toReview={summary.toReview}
        onSelect={(tab) => void go(tab)}
      />
      <ExportDialog
        characterId={characterId}
        opened={exporting}
        onClose={() => setExporting(false)}
        onPrint={() =>
          navigate({
            to: "/print/$characterId",
            params: { characterId: String(characterId) },
          })
        }
      />
      <Modal
        opened={confirming}
        onClose={() => setConfirming(false)}
        title={t("character.removeTitle", { name })}
      >
        <Stack>
          <Text size="sm">{t("character.removeBody")}</Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setConfirming(false)}>
              {t("character.cancel")}
            </Button>
            <Button
              color="red"
              loading={remove.isPending}
              onClick={() => remove.mutate()}
            >
              {t("character.removeConfirm")}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </>
  );
}
