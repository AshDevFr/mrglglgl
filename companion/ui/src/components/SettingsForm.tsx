import { Alert, Button, Group, Select, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { getSettings, listNarrators } from "@/api/commands";
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
  /** The settings as stored now, which each save starts from. */
  readStored?: () => Promise<Settings>;
}

const storedSettings = async () => (await getSettings()).settings;

const SYSTEM = "system";
const RETENTIONS: BackupRetention[] = ["days30", "days90", "year", "forever"];

/** The endpoint's own fields: saved together, only when the player says so. */
type EndpointPart = Pick<
  WritingSettings,
  "provider" | "endpointUrl" | "endpointModel" | "pseudonyms"
>;
const endpointPart = (w: WritingSettings): EndpointPart => ({
  provider: w.provider,
  endpointUrl: w.endpointUrl,
  endpointModel: w.endpointModel,
  pseudonyms: w.pseudonyms,
});
const samePart = (a: EndpointPart, b: EndpointPart) =>
  a.provider === b.provider &&
  (a.endpointUrl ?? "") === (b.endpointUrl ?? "") &&
  (a.endpointModel ?? "") === (b.endpointModel ?? "") &&
  a.pseudonyms === b.pseudonyms;

/** How long typing in the writing preferences waits before it is saved. */
const TYPING_PAUSE_MS = 700;

/**
 * A settings tab's form. Every choice is saved as soon as it is made, except who writes: an
 * endpoint half typed in would have background writing fail, so its fields wait for their own
 * Save.
 */
export function SettingsForm({
  view,
  onSave,
  section = "general",
  readStored = storedSettings,
}: Props) {
  const { t } = useTranslation();
  const [folder, setFolder] = useState(view.settings.wowRootOverride ?? "");
  const [retention, setRetention] = useState<BackupRetention>(
    view.settings.backupRetention,
  );
  const [language, setLanguage] = useState(view.settings.uiLanguage ?? SYSTEM);
  const [writing, setWriting] = useState<WritingSettings>(
    view.settings.writing,
  );
  const [endpoint, setEndpoint] = useState<WritingSettings>(
    view.settings.writing,
  );
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
    enabled: section === "writing",
  });
  const [state, setState] = useState<"idle" | "saving" | "saved">("idle");
  const [error, setError] = useState<string | null>(null);

  // Saves run one after another, each on the settings as stored at that moment: a choice made
  // elsewhere meanwhile (a model picked above, another tab) is kept, not undone by this form.
  const queue = useRef<Promise<void>>(Promise.resolve());
  function persist(change: (stored: Settings) => Settings) {
    setError(null);
    setState("saving");
    queue.current = queue.current.then(async () => {
      try {
        await onSave(change(await readStored()));
        setState("saved");
      } catch (e) {
        setError(String(e));
        setState("idle");
      }
    });
  }
  const persistWriting = (patch: Partial<WritingSettings>) =>
    persist((s) => ({ ...s, writing: { ...s.writing, ...patch } }));

  // Typing in the preferences saves once the player pauses, and when the form goes away.
  const typed = useRef<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const flushTyped = useRef(() => {});
  flushTyped.current = () => {
    clearTimeout(timer.current);
    const preferences = typed.current;
    if (preferences === null) return;
    typed.current = null;
    persistWriting({ preferences });
  };
  useEffect(() => () => flushTyped.current(), []);

  function changeWriting(next: WritingSettings) {
    const patch = Object.fromEntries(
      Object.entries(next).filter(
        ([key, value]) => writing[key as keyof WritingSettings] !== value,
      ),
    ) as Partial<WritingSettings>;
    setWriting(next);
    if ("preferences" in patch) {
      typed.current = next.preferences;
      clearTimeout(timer.current);
      timer.current = setTimeout(() => flushTyped.current(), TYPING_PAUSE_MS);
      delete patch.preferences;
    }
    if (Object.keys(patch).length > 0) persistWriting(patch);
  }

  const endpointChanged = !samePart(
    endpointPart(endpoint),
    endpointPart(view.settings.writing),
  );

  return (
    <Stack maw={640}>
      {section === "general" ? (
        <>
          <WowFolderField
            value={folder}
            saved={folder.trim() === "" ? null : folder.trim()}
            roots={view.roots}
            onChange={(next) => {
              setFolder(next);
              persist((s) => ({
                ...s,
                wowRootOverride: next.trim() === "" ? null : next.trim(),
              }));
            }}
          />
          <Select
            label={t("settings.backupRetention")}
            description={t("settings.backupRetentionHelp")}
            data={RETENTIONS.map((value) => ({
              value,
              label: t(`settings.retention.${value}`),
            }))}
            value={retention}
            onChange={(v) => {
              const next = (v as BackupRetention | null) ?? retention;
              setRetention(next);
              persist((s) => ({ ...s, backupRetention: next }));
            }}
            allowDeselect={false}
          />
          <Select
            label={t("settings.language")}
            data={[
              { value: SYSTEM, label: t("settings.languageSystem") },
              { value: "en", label: t("settings.languageEnglish") },
              { value: "fr", label: t("settings.languageFrench") },
            ]}
            value={language}
            onChange={(v) => {
              const next = v ?? SYSTEM;
              setLanguage(next);
              persist((s) => ({
                ...s,
                uiLanguage: next === SYSTEM ? null : next,
              }));
            }}
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
            onChange={changeWriting}
            narrators={narrators.data ?? []}
          />
          <EndpointFields value={endpoint} onChange={setEndpoint} />
          {endpointChanged && (
            <Group gap="xs">
              <Button
                w="fit-content"
                onClick={() => persistWriting(endpointPart(endpoint))}
              >
                {t("settings.saveEndpoint")}
              </Button>
              <Button
                variant="subtle"
                color="gray"
                onClick={() => setEndpoint(view.settings.writing)}
              >
                {t("settings.discardEndpoint")}
              </Button>
              <Text size="xs" c="dimmed">
                {t("settings.endpointUnsaved")}
              </Text>
            </Group>
          )}
        </>
      )}
      {error && (
        <Alert color="red">
          {problem(t, "settings.error", { message: error })}
        </Alert>
      )}
      <Text size="xs" c="dimmed" role="status">
        {state === "saving"
          ? t("settings.saving")
          : state === "saved"
            ? t("settings.savedAuto")
            : t("settings.autosave")}
      </Text>
    </Stack>
  );
}
