import { Alert, Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  characterJournal,
  characterMilestones,
  characterScreenshots,
  deleteCharacter,
  listCharacters,
} from "@/api/commands";
import { CharacterHeader } from "@/components/CharacterHeader";
import { EmptyState } from "@/components/EmptyState";
import { JournalPage } from "@/components/JournalPage";
import { WeekNav } from "@/components/WeekNav";
import { daysNewestFirst, displayName } from "@/lib/journal";
import { dayKey } from "@/lib/weeks";

interface Props {
  characterId: number;
  day?: string;
  /** Today, "YYYY-MM-DD"; injectable for tests. */
  today?: string;
}

export function CharacterPage({
  characterId,
  day,
  today = dayKey(new Date()),
}: Props) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);

  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const journal = useQuery({
    queryKey: ["journal", characterId],
    queryFn: () => characterJournal(characterId),
  });
  const awards = useQuery({
    queryKey: ["milestones", characterId],
    queryFn: () => characterMilestones(characterId),
  });
  const screenshots = useQuery({
    queryKey: ["screenshots", characterId],
    queryFn: () => characterScreenshots(characterId),
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
  if (journal.isPending || !summary) {
    return (
      <div className="desk" style={{ padding: "2rem" }}>
        <EmptyState title={t("loading")} />
      </div>
    );
  }
  const days = journal.data ? daysNewestFirst(journal.data) : [];
  const written = new Set(days);
  const selected = day ?? days[0] ?? today;
  const name = displayName(summary.character);
  const status = summary.character.status;

  function show(target: string) {
    navigate({
      to: "/characters/$characterId",
      params: { characterId: String(characterId) },
      search: { day: target },
    });
  }

  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <Stack gap="lg" maw="48rem">
        <Group justify="space-between" align="flex-start" wrap="nowrap">
          <CharacterHeader
            character={summary.character}
            journal={journal.data ?? null}
          />
          <Group gap="xs" wrap="nowrap">
            <Button
              variant="subtle"
              color="gray"
              onClick={() =>
                navigate({
                  to: "/characters/$characterId/milestones",
                  params: { characterId: String(characterId) },
                })
              }
            >
              {t("nav.milestones")}
            </Button>
            <Button
              variant="subtle"
              color="red"
              onClick={() => setConfirming(true)}
            >
              {t("character.remove")}
            </Button>
          </Group>
        </Group>
        {summary.archived && <Alert>{t("sync.archivedHelp")}</Alert>}
        {status && <Alert color="yellow">{t(`sync.status.${status}`)}</Alert>}
        <WeekNav
          selected={selected}
          written={written}
          today={today}
          onSelect={show}
        />
        {journal.data && written.has(selected) ? (
          <JournalPage
            journal={journal.data}
            day={selected}
            awards={awards.data ?? []}
            screenshots={screenshots.data?.[selected] ?? []}
          />
        ) : (
          <EmptyState title={t("week.empty")}>{t("week.emptyHelp")}</EmptyState>
        )}
      </Stack>
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
    </div>
  );
}
