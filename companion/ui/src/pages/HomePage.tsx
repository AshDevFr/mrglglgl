import { Alert, Button, Group, Paper, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { characterStories, listCharacters, models } from "@/api/commands";
import type { CharacterSummary } from "@/bindings/CharacterSummary";
import { AddonSummary } from "@/components/AddonSummary";
import { EmptyState } from "@/components/EmptyState";
import { FirstRun } from "@/components/FirstRun";
import { MurlocArt } from "@/components/MurlocArt";
import { problem } from "@/i18n/problems";
import { formatDay } from "@/lib/format";
import { displayName } from "@/lib/journal";
import { rememberSettingsTab } from "@/lib/settingsTab";

/** How many characters the home page offers to pick up where they left off. */
const RECENT = 4;

/** A character's newest written day, to read on from. */
function LatestEntry({
  summary,
  onOpen,
}: {
  summary: CharacterSummary;
  onOpen: (characterId: number, day?: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const id = summary.character.id;
  const stories = useQuery({
    queryKey: ["stories", id],
    queryFn: () => characterStories(id),
  });
  const latest = (stories.data ?? [])
    .filter(
      (s) =>
        s.kind === "day" &&
        s.status === "accepted" &&
        s.check !== "failed" &&
        !s.partial,
    )
    .sort((a, b) => b.startDay.localeCompare(a.startDay) || b.id - a.id)[0];
  // The first paragraph, without the murloc's margin note.
  const opening = latest?.text
    .split(/\n\s*\n/)
    .map((p) => p.trim())
    .find((p) => p !== "" && !p.startsWith("*"));
  const name = displayName(summary.character);
  return (
    <Paper className="latest-entry" p="md" withBorder>
      <Stack gap={6}>
        <Text fw={600} className="latest-entry-name">
          {name}
        </Text>
        {latest ? (
          <>
            <Text size="xs" c="dimmed">
              {formatDay(latest.startDay, i18n.language, true)}
            </Text>
            <Text className="latest-entry-text" lineClamp={3}>
              {opening}
            </Text>
          </>
        ) : (
          <Text size="sm" c="dimmed">
            {t("home.nothingWritten")}
          </Text>
        )}
        <Button
          size="xs"
          variant="light"
          w="fit-content"
          aria-label={t("home.continueFor", { name })}
          onClick={() => onOpen(id, latest?.startDay)}
        >
          {t("home.continue")}
        </Button>
      </Stack>
    </Paper>
  );
}

export function HomePage({
  onOpenSettings = () => {},
  onOpenCharacter = () => {},
}: {
  /** Opens the settings, where models are downloaded. */
  onOpenSettings?: () => void;
  /** Opens a character's journal, on a day when one is given. */
  onOpenCharacter?: (characterId: number, day?: string) => void;
}) {
  const { t } = useTranslation();
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const catalogue = useQuery({ queryKey: ["models"], queryFn: models });
  const empty = characters.isSuccess && characters.data.length === 0;
  // The first steps stay on the home page until there are journals and a model to write them.
  const noModel =
    catalogue.isSuccess && !catalogue.data.models.some((m) => m.installed);

  return (
    <div className="desk" style={{ padding: "2rem 2.5rem" }}>
      <Stack maw="40rem">
        <Group justify="space-between" align="center" wrap="nowrap">
          <h1 className="character-name">{t("app.title")}</h1>
          <MurlocArt pose="scribe" size={150} />
        </Group>
        {characters.isPending && <EmptyState title={t("loading")} />}
        {characters.isError && (
          <Alert color="red">
            {problem(t, "errors.load", { message: String(characters.error) })}
          </Alert>
        )}
        {empty && (
          <EmptyState title={t("home.empty")} art="fishing">
            {t("home.emptyHelp")}
          </EmptyState>
        )}
        <AddonSummary
          onOpen={() => {
            rememberSettingsTab("addon");
            onOpenSettings();
          }}
        />
        {characters.isSuccess && !empty && (
          <Stack gap="sm">
            <h2 className="page-title page-title-small">{t("home.latest")}</h2>
            {characters.data
              .filter((c) => c.character.enabled)
              .sort(
                (a, b) =>
                  (b.character.lastSynced ?? 0) - (a.character.lastSynced ?? 0),
              )
              .slice(0, RECENT)
              .map((c) => (
                <LatestEntry
                  key={c.character.id}
                  summary={c}
                  onOpen={onOpenCharacter}
                />
              ))}
          </Stack>
        )}
        {(empty || noModel) && <FirstRun onOpenSettings={onOpenSettings} />}
        {!empty && !noModel && (
          <Text size="sm" c="dimmed">
            {t("sync.explain")}
          </Text>
        )}
      </Stack>
    </div>
  );
}
