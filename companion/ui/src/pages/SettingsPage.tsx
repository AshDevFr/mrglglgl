import { Alert, Stack, Tabs, Text, Title } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { appInfo, getSettings, saveSettings } from "@/api/commands";
import type { Settings } from "@/bindings/Settings";
import { AddonPanel } from "@/components/AddonPanel";
import { CharactersPanel } from "@/components/CharactersPanel";
import { LogPanel } from "@/components/LogPanel";
import { ModelsPanel } from "@/components/ModelsPanel";
import { NarratorsPanel } from "@/components/NarratorsPanel";
import { PaperField } from "@/components/PaperField";
import { SettingsForm } from "@/components/SettingsForm";
import { UpdateCheck } from "@/components/UpdateCheck";
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
  const info = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
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
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <h1 className="page-title">{t("settings.title")}</h1>
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
              {/* What applies at once on this computer, apart from what Save keeps. */}
              <Stack gap="md">
                <Title order={2} size="h3">
                  {t("settings.thisComputer")}
                </Title>
                <Text size="sm" c="dimmed">
                  {t("settings.thisComputerHelp")}
                </Text>
                <PaperField />
                <ZoomField />
                <WowheadField />
              </Stack>
              <Stack gap="md">
                <Title order={2} size="h3">
                  {t("settings.savedSection")}
                </Title>
                <Text size="sm" c="dimmed">
                  {t("settings.savedSectionHelp")}
                </Text>
                <SettingsForm
                  view={view.data}
                  onSave={save}
                  section="general"
                />
              </Stack>
              <Stack gap="xs">
                <Title order={2} size="h3">
                  {t("settings.thisApp")}
                </Title>
                {info.data && (
                  <Text size="sm">
                    {t("app.version", { version: info.data.version })}
                  </Text>
                )}
                <UpdateCheck />
              </Stack>
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="ai">
            <Stack gap="xl" maw={640}>
              <SettingsForm
                view={view.data}
                onSave={save}
                section="ai"
                localModels={<ModelsPanel onChoose={(id) => void choose(id)} />}
              />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="writing">
            <Stack gap="xl" maw={640}>
              <SettingsForm view={view.data} onSave={save} section="writing" />
            </Stack>
          </Tabs.Panel>
          <Tabs.Panel value="characters">
            <Stack gap="xl" maw={720}>
              <CharactersPanel />
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
    </div>
  );
}
