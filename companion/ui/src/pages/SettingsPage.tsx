import { Container, Stack, Text, Title } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { getSettings, saveSettings } from "@/api/commands";
import type { Settings } from "@/bindings/Settings";
import { ModelsPanel } from "@/components/ModelsPanel";
import { SettingsForm } from "@/components/SettingsForm";
import { applyLanguage } from "@/i18n";

export function SettingsPage() {
  const { t } = useTranslation();
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
      {view.data ? (
        <Stack gap="xl" maw={640}>
          <ModelsPanel onChoose={(id) => void choose(id)} />
          <SettingsForm view={view.data} onSave={save} />
        </Stack>
      ) : (
        <Text>{t("loading")}</Text>
      )}
    </Container>
  );
}
