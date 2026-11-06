import { Alert, Button, Select, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { listNarrators } from "@/api/commands";
import type { BackupRetention } from "@/bindings/BackupRetention";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import type { WritingSettings } from "@/bindings/WritingSettings";
import { problem } from "@/i18n/problems";
import { EndpointFields } from "./EndpointFields";
import { WowFolderField } from "./WowFolderField";
import { WritingFields } from "./WritingFields";

interface Props {
  view: SettingsView;
  onSave: (settings: Settings) => Promise<unknown>;
  /** Which settings this form edits; everything else is saved as it is. */
  section?: "general" | "writing";
}

const SYSTEM = "system";
const RETENTIONS: BackupRetention[] = ["days30", "days90", "year", "forever"];

export function SettingsForm({ view, onSave, section = "general" }: Props) {
  const { t } = useTranslation();
  const [folder, setFolder] = useState(view.settings.wowRootOverride ?? "");
  const [retention, setRetention] = useState<BackupRetention>(
    view.settings.backupRetention,
  );
  const [language, setLanguage] = useState(view.settings.uiLanguage ?? SYSTEM);
  const [writing, setWriting] = useState<WritingSettings>(
    view.settings.writing,
  );
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
    enabled: section === "writing",
  });
  const [state, setState] = useState<"idle" | "saving" | "saved">("idle");
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setState("saving");
    setError(null);
    try {
      // Each tab saves its own settings and keeps the rest as stored, so an edit left unsaved
      // on another tab is neither saved nor lost by this one.
      await onSave(
        section === "general"
          ? {
              ...view.settings,
              wowRootOverride: folder.trim() === "" ? null : folder.trim(),
              backupRetention: retention,
              uiLanguage: language === SYSTEM ? null : language,
            }
          : { ...view.settings, writing },
      );
      setState("saved");
    } catch (e) {
      setError(String(e));
      setState("idle");
    }
  }

  return (
    <Stack maw={640}>
      {section === "general" ? (
        <>
          <WowFolderField
            value={folder}
            saved={view.settings.wowRootOverride}
            roots={view.roots}
            onChange={setFolder}
          />
          <Select
            label={t("settings.backupRetention")}
            description={t("settings.backupRetentionHelp")}
            data={RETENTIONS.map((value) => ({
              value,
              label: t(`settings.retention.${value}`),
            }))}
            value={retention}
            onChange={(v) =>
              setRetention((v as BackupRetention | null) ?? retention)
            }
            allowDeselect={false}
          />
          <Select
            label={t("settings.language")}
            data={[
              { value: SYSTEM, label: t("settings.languageSystem") },
              { value: "en", label: t("settings.languageEnglish") },
            ]}
            value={language}
            onChange={(v) => setLanguage(v ?? SYSTEM)}
            allowDeselect={false}
          />
          <Text size="sm" c="dimmed">
            {t("sync.explain")}
          </Text>
          <Text size="sm" c="dimmed">
            {t("settings.forgetting")}
          </Text>
        </>
      ) : (
        <>
          <WritingFields
            value={writing}
            onChange={setWriting}
            narrators={narrators.data ?? []}
          />
          <EndpointFields value={writing} onChange={setWriting} />
        </>
      )}
      {error && (
        <Alert color="red">
          {problem(t, "settings.error", { message: error })}
        </Alert>
      )}
      {state === "saved" && <Alert color="green">{t("settings.saved")}</Alert>}
      <Button onClick={save} loading={state === "saving"} w="fit-content">
        {t("settings.save")}
      </Button>
    </Stack>
  );
}
