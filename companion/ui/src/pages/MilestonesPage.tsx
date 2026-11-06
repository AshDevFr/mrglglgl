import { Group, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { characterMilestones } from "@/api/commands";
import type { Award } from "@/bindings/Award";
import { CharacterTop } from "@/components/CharacterTop";
import { EmptyState } from "@/components/EmptyState";
import { InkStamp } from "@/components/InkStamp";
import { formatDay } from "@/lib/format";

const TILTS = [-6, 4, -2, 7, -4];

export function MilestoneList({ awards }: { awards: Award[] }) {
  const { t, i18n } = useTranslation();
  if (awards.length === 0)
    return <EmptyState title={t("milestones.none")} art="wave" />;
  return (
    <Stack gap="lg">
      {awards.map((award, i) => (
        <Group key={award.id} gap="xl" wrap="nowrap">
          <InkStamp award={award} size={96} tilt={TILTS[i % TILTS.length]} />
          <Stack gap={2}>
            <Text className="entry" fw={600}>
              {t(`milestones.${award.id}.name`)}
            </Text>
            <Text className="entry">
              {t(`milestones.${award.id}.description`)}
            </Text>
            <Text size="sm" c="dimmed">
              {t("milestones.earned", {
                day: formatDay(award.day, i18n.language),
              })}
            </Text>
          </Stack>
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
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <Stack gap="lg" maw="48rem" mb="xl">
        <CharacterTop characterId={characterId} current="milestones" />
      </Stack>
      <article className="journal-page">
        <h2>{t("milestones.title")}</h2>
        {awards.isPending ? (
          <EmptyState title={t("loading")} />
        ) : (
          <MilestoneList awards={awards.data ?? []} />
        )}
      </article>
    </div>
  );
}
