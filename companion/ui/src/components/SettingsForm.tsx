import {
  Alert,
  Button,
  List,
  NumberInput,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";

interface Props {
  view: SettingsView;
  onSave: (settings: Settings) => Promise<unknown>;
}

const SYSTEM = "system";

export function SettingsForm({ view, onSave }: Props) {
  const { t } = useTranslation();
  const [folder, setFolder] = useState(view.settings.wowRootOverride ?? "");
  const [backups, setBackups] = useState<number>(view.settings.backupCount);
  const [language, setLanguage] = useState(view.settings.uiLanguage ?? SYSTEM);
  const [state, setState] = useState<"idle" | "saving" | "saved">("idle");
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setState("saving");
    setError(null);
    try {
      await onSave({
        wowRootOverride: folder.trim() === "" ? null : folder.trim(),
        backupCount: backups,
        uiLanguage: language === SYSTEM ? null : language,
      });
      setState("saved");
    } catch (e) {
      setError(String(e));
      setState("idle");
    }
  }

  return (
    <Stack maw={640}>
      <TextInput
        label={t("settings.wowFolder")}
        description={t("settings.wowFolderHelp")}
        value={folder}
        onChange={(e) => setFolder(e.currentTarget.value)}
      />
      {view.roots.length === 0 ? (
        <Text size="sm">{t("settings.foundNone")}</Text>
      ) : (
        <div>
          <Text size="sm">
            {t("settings.found", { count: view.roots.length })}
          </Text>
          <List size="sm">
            {view.roots.map((root) => (
              <List.Item key={root.path}>{root.path}</List.Item>
            ))}
          </List>
        </div>
      )}
      <NumberInput
        label={t("settings.backupCount")}
        description={t("settings.backupCountHelp")}
        min={1}
        max={500}
        value={backups}
        onChange={(v) => setBackups(typeof v === "number" ? v : backups)}
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
      {error && (
        <Alert color="red">{t("settings.error", { message: error })}</Alert>
      )}
      {state === "saved" && <Alert color="green">{t("settings.saved")}</Alert>}
      <Button onClick={save} loading={state === "saving"} w="fit-content">
        {t("settings.save")}
      </Button>
    </Stack>
  );
}
