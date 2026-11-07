import { Alert, Container, Stack, Tabs, Text, Title } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
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
import { WowheadField } from "@/components/WowheadField";
import { ZoomField } from "@/components/ZoomField";
import { applyLanguage } from "@/i18n";
import { problem } from "@/i18n/problems";
import {
  rememberedSettingsTab,
  rememberSettingsTab,
  SETTINGS_TABS,
  type SettingsTab,
} from "@/lib/settingsTab";

const TABS = SETTINGS_TABS;
type Tab = SettingsTab;

export function SettingsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const view = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const [tab, setTab] = useState<Tab>(rememberedSettingsTab);

  function openTab(value: string | null) {
    const next = TABS.find((t) => t === value) ?? "general";
    setTab(next);
    rememberSettingsTab(next);
  }

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
        <Tabs value={tab} onChange={openTab} keepMounted={false}>
          <Tabs.List mb="lg">
            {TABS.map((value) => (
              <Tabs.Tab key={value} value={value}>
                {t(`settings.tab.${value}`)}
              </Tabs.Tab>
            ))}
          </Tabs.List>
          <Tabs.Panel value="general">
            <Stack gap="xl" maw={640}>
              <PaperField />
              <ZoomField />
              <WowheadField />
              <SettingsForm view={view.data} onSave={save} section="general" />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="writing">
            <Stack gap="xl" maw={640}>
              <ModelsPanel onChoose={(id) => void choose(id)} />
              <SettingsForm view={view.data} onSave={save} section="writing" />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="narrators">
            <Stack gap="xl" maw={640}>
              <NarratorsPanel
                onEdit={(narratorId) =>
                  navigate({
                    to: "/narrators/$narratorId",
                    params: { narratorId },
                  })
                }
              />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="website">
            <Stack gap="xl" maw={640}>
              <WebsitePanel site={view.data.settings.site} />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="addon">
            <Stack gap="xl" maw={640}>
              <AddonPanel />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="logs">
            <Stack gap="xl" maw={880}>
              <LogPanel />
            </Stack>
          </Tabs.Panel>
        </Tabs>
      ) : (
        <Text>{t("loading")}</Text>
      )}
    </Container>
  );
}
