import { Group, Paper, Stack, Text, Title } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { characterMilestones } from "@/api/commands";
import type { Award } from "@/bindings/Award";
import { MilestoneStamp } from "@/components/MilestoneStamp";

export function MilestoneList({ awards }: { awards: Award[] }) {
  const { t } = useTranslation();
  if (awards.length === 0)
    return <Text className="reading">{t("milestones.none")}</Text>;
  return (
    <Stack>
      {awards.map((award) => (
        <Group key={award.id} gap="lg" wrap="nowrap">
          <MilestoneStamp award={award} showDay />
          <Text className="reading">
            {t(`milestones.${award.id}.description`)}
          </Text>
        </Group>
      ))}
    </Stack>
  );
}

export function MilestonesPage({ characterId }: { characterId: number }) {
  const { t } = useTranslation();
  const awards = useQuery({
    queryKey: ["milestones", characterId],
    queryFn: () => characterMilestones(characterId),
  });
  return (
    <Paper className="paper" m="md" p="xl">
      <Stack className="reading">
        <Title order={1}>{t("milestones.title")}</Title>
        {awards.isPending ? (
          <Text>{t("loading")}</Text>
        ) : (
          <MilestoneList awards={awards.data ?? []} />
        )}
      </Stack>
    </Paper>
  );
}
