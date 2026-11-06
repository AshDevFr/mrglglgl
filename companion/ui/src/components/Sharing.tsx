import {
  Alert,
  Button,
  Group,
  Modal,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  copyEntryText,
  defaultExportFolder,
  exportCharacter,
} from "@/api/commands";
import type { ExportedFiles } from "@/bindings/ExportedFiles";
import type { Story } from "@/bindings/Story";
import { problem } from "@/i18n/problems";

/** Copies an entry, with its day and the footer, as plain text. */
export function CopyEntryButton({ story }: { story: Story }) {
  const { t } = useTranslation();
  const [state, setState] = useState<"idle" | "copied" | "failed">("idle");
  async function copy() {
    try {
      const text = await copyEntryText(story.characterId, story.id);
      await navigator.clipboard.writeText(text);
      setState("copied");
    } catch {
      setState("failed");
    }
  }
  return (
    <>
      <Button
        size="xs"
        variant="subtle"
        color="gray"
        onClick={() => void copy()}
      >
        {state === "copied" ? t("share.copied") : t("share.copy")}
      </Button>
      {state === "failed" && (
        <Text size="xs" c="red" role="status">
          {problem(t, "share.copyFailed")}
        </Text>
      )}
    </>
  );
}

/** Saves the whole journal as Markdown and as one HTML page, in a folder the player picks. */
export function ExportDialog({
  characterId,
  opened,
  onClose,
}: {
  characterId: number;
  opened: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<ExportedFiles | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!opened) return;
    setDone(null);
    setError(null);
    void defaultExportFolder().then((f) =>
      setFolder((current) => current || f || ""),
    );
  }, [opened]);

  async function save() {
    setBusy(true);
    setError(null);
    try {
      setDone(await exportCharacter(characterId, folder));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal opened={opened} onClose={onClose} title={t("share.exportTitle")}>
      <Stack>
        <Text size="sm">{t("share.exportHelp")}</Text>
        <TextInput
          label={t("share.folder")}
          value={folder}
          onChange={(e) => setFolder(e.currentTarget.value)}
        />
        {error && (
          <Alert color="red">
            {problem(t, "share.exportFailed", { message: error })}
          </Alert>
        )}
        {done && (
          <Alert color="green">
            {t("share.exported", { markdown: done.markdown, html: done.html })}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {t("share.close")}
          </Button>
          <Button
            loading={busy}
            disabled={folder.trim() === ""}
            onClick={() => void save()}
          >
            {t("share.export")}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
