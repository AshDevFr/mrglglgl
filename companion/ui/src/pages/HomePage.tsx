import { Alert, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { appInfo, listCharacters, models } from "@/api/commands";
import { AddonSummary } from "@/components/AddonSummary";
import { EmptyState } from "@/components/EmptyState";
import { FirstRun } from "@/components/FirstRun";
import { problem } from "@/i18n/problems";
import { rememberSettingsTab } from "@/lib/settingsTab";

export function HomePage({
  onOpenSettings = () => {},
}: {
  /** Opens the settings, where models are downloaded. */
  onOpenSettings?: () => void;
}) {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
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
        <h1 className="character-name">{t("app.title")}</h1>
        {characters.isPending && <EmptyState title={t("loading")} />}
        {characters.isError && (
          <Alert color="red">
            {problem(t, "errors.load", { message: String(characters.error) })}
          </Alert>
        )}
        {empty && (
          <EmptyState title={t("home.empty")}>{t("home.emptyHelp")}</EmptyState>
        )}
        <AddonSummary
          onOpen={() => {
            rememberSettingsTab("addon");
            onOpenSettings();
          }}
        />
        {characters.isSuccess && !empty && <Text>{t("home.pick")}</Text>}
        {(empty || noModel) && <FirstRun onOpenSettings={onOpenSettings} />}
        {!empty && !noModel && (
          <Text size="sm" c="dimmed">
            {t("sync.explain")}
          </Text>
        )}
        {info.data && (
          <Text size="sm" c="dimmed">
            {t("app.version", { version: info.data.version })}
          </Text>
        )}
      </Stack>
    </div>
  );
}
