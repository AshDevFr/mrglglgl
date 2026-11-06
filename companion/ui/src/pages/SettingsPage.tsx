import { Alert, Container, Stack, Text, Title } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { getSettings, saveSettings } from "@/api/commands";
import type { Settings } from "@/bindings/Settings";
import { AddonPanel } from "@/components/AddonPanel";
import { LogPanel } from "@/components/LogPanel";
import { ModelsPanel } from "@/components/ModelsPanel";
import { NarratorsPanel } from "@/components/NarratorsPanel";
import { PaperField } from "@/components/PaperField";
import { SettingsForm } from "@/components/SettingsForm";
import { WebsitePanel } from "@/components/WebsitePanel";
import { ZoomField } from "@/components/ZoomField";
import { applyLanguage } from "@/i18n";
import { problem } from "@/i18n/problems";

export function SettingsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const view = useQuery({ queryKey: ["settings"], queryFn: getSettings });

  async function save(settings: Settings) {
    await saveSettings(settings);
    await applyLanguage(settings.uiLanguage);
    await queryClient.invalidateQueries();
  }

  async function choose(modelId: string) {
    if (!view.data) return;
    const settings = view.data.settings;
    await save({ ...settings, writing: { ...settings.writing, modelId } });
  }

  return (
    <Container py="xl">
      <Title order={1} mb="md">
        {t("settings.title")}
      </Title>
      {view.isError && (
        <Alert color="red">
          {problem(t, "errors.load", { message: String(view.error) })}
        </Alert>
      )}
      {view.data && view.data.unreadable.length > 0 && (
        <Alert color="orange" mb="md" maw={640}>
          {problem(t, "settings.unreadable", {
            list: view.data.unreadable
              .map((key) => t(`settings.group.${key}`, { defaultValue: key }))
              .join(", "),
          })}
        </Alert>
      )}
      {view.data ? (
        <Stack gap="xl" maw={640}>
          <PaperField />
          <ZoomField />
          <ModelsPanel onChoose={(id) => void choose(id)} />
          <SettingsForm view={view.data} onSave={save} />
          <AddonPanel />
          <NarratorsPanel
            onEdit={(narratorId) =>
              navigate({
                to: "/narrators/$narratorId",
                params: { narratorId },
              })
            }
          />
          <WebsitePanel site={view.data.settings.site} />
          <LogPanel />
        </Stack>
      ) : (
        <Text>{t("loading")}</Text>
      )}
    </Container>
  );
}
