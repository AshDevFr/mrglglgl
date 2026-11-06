import {
  Alert,
  Button,
  Group,
  Modal,
  NavLink,
  Paper,
  ScrollArea,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  characterJournal,
  characterMilestones,
  deleteCharacter,
  listCharacters,
} from "@/api/commands";
import type { Award } from "@/bindings/Award";
import type { Journal } from "@/bindings/Journal";
import { DayFactsList } from "@/components/DayFactsList";
import { MilestoneStamp } from "@/components/MilestoneStamp";
import { formatDay } from "@/lib/format";
import { dayFacts, daysNewestFirst, displayName } from "@/lib/journal";

interface DayViewProps {
  journal: Journal;
  day: string;
  awards: Award[];
}

/** One day: its facts and the stamps earned that day. */
export function DayView({ journal, day, awards }: DayViewProps) {
  const { t, i18n } = useTranslation();
  const facts = dayFacts(journal, journal.days[day]);
  const stamps = awards.filter((a) => a.day === day);
  return (
    <Paper className="paper" p="xl">
      <Stack className="reading">
        <Title order={2}>{formatDay(day, i18n.language, true)}</Title>
        {stamps.length > 0 && (
          <Group>
            {stamps.map((a) => (
              <MilestoneStamp key={a.id} award={a} />
            ))}
          </Group>
        )}
        <Title order={4}>{t("day.facts")}</Title>
        <DayFactsList facts={facts} />
        {facts.compacted && (
          <Text size="sm" c="dimmed">
            {t("character.compacted")}
          </Text>
        )}
      </Stack>
    </Paper>
  );
}

interface Props {
  characterId: number;
  day?: string;
}

export function CharacterPage({ characterId, day }: Props) {
  const { t, i18n } = useTranslation();
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
  const remove = useMutation({
    mutationFn: () => deleteCharacter(characterId),
    onSuccess: async () => {
      setConfirming(false);
      await queryClient.invalidateQueries();
      await navigate({ to: "/" });
    },
  });

  const summary = characters.data?.find((c) => c.character.id === characterId);
  if (journal.isPending || !summary) return <Text p="xl">{t("loading")}</Text>;
  const name = displayName(summary.character);
  const status = summary.character.status;
  const days = journal.data ? daysNewestFirst(journal.data) : [];
  const selected = day && days.includes(day) ? day : days[0];

  return (
    <Stack p="md" h="100%">
      <Group justify="space-between">
        <Title order={1} className="reading">
          {name}
        </Title>
        <Group>
          <Button
            variant="subtle"
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
      {days.length === 0 || !journal.data ? (
        <Text>{t("character.noDays")}</Text>
      ) : (
        <Group
          align="flex-start"
          wrap="nowrap"
          gap="md"
          style={{ flex: 1, minHeight: 0 }}
        >
          <ScrollArea w={200} h="100%">
            {days.map((d) => (
              <NavLink
                key={d}
                label={formatDay(d, i18n.language)}
                active={d === selected}
                onClick={() =>
                  navigate({
                    to: "/characters/$characterId",
                    params: { characterId: String(characterId) },
                    search: { day: d },
                  })
                }
              />
            ))}
          </ScrollArea>
          <ScrollArea style={{ flex: 1 }} h="100%">
            {selected && (
              <DayView
                journal={journal.data}
                day={selected}
                awards={awards.data ?? []}
              />
            )}
          </ScrollArea>
        </Group>
      )}
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
    </Stack>
  );
}
