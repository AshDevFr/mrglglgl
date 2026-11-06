import {
  Alert,
  Button,
  List,
  Select,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { listNarrators } from "@/api/commands";
import type { BackupRetention } from "@/bindings/BackupRetention";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import type { WritingSettings } from "@/bindings/WritingSettings";
import { EndpointFields } from "./EndpointFields";
import { WritingFields } from "./WritingFields";

interface Props {
  view: SettingsView;
  onSave: (settings: Settings) => Promise<unknown>;
}

const SYSTEM = "system";
const RETENTIONS: BackupRetention[] = ["days30", "days90", "year", "forever"];

export function SettingsForm({ view, onSave }: Props) {
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
  });
  const [state, setState] = useState<"idle" | "saving" | "saved">("idle");
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setState("saving");
    setError(null);
    try {
      await onSave({
        wowRootOverride: folder.trim() === "" ? null : folder.trim(),
        backupRetention: retention,
        uiLanguage: language === SYSTEM ? null : language,
        writing,
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
      <WritingFields
        value={writing}
        onChange={setWriting}
        narrators={narrators.data ?? []}
      />
      <EndpointFields value={writing} onChange={setWriting} />
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
