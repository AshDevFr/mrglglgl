import {
  Alert,
  Button,
  Group,
  Menu,
  Modal,
  Radio,
  Stack,
  Text,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  copyEntryText,
  exportCharacter,
  exportDebug,
  showInFolder,
} from "@/api/commands";
import type { Story } from "@/bindings/Story";
import { problem } from "@/i18n/problems";
import { exportFolder, rememberExportFolder } from "@/lib/exportFolder";
import { exportPage, loadExportData } from "@/lib/exportPage";
import { FolderField } from "./FolderField";

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

/** Writes the day or week's debug file, as JSON, into a folder the player picks: what was
 * recorded, the facts and the prompt the narrator is given, and every version written. A menu
 * item closes as soon as it is chosen, so what happened is told in a notification. */
export function DebugExportItem({ story }: { story: Story }) {
  const { t } = useTranslation();
  async function save() {
    const folder = await open({
      directory: true,
      defaultPath: await exportFolder(),
    });
    if (typeof folder !== "string") return;
    try {
      const saved = await exportDebug(
        story.characterId,
        story.kind,
        story.startDay,
        folder,
      );
      rememberExportFolder(folder);
      notifications.show({
        message: (
          <Group gap={4}>
            <Text size="sm">{t("share.debugSaved", { path: saved })}</Text>
            <Button
              size="compact-xs"
              variant="subtle"
              color="gray"
              onClick={() => void showInFolder(saved)}
            >
              {t("share.showInFolder")}
            </Button>
          </Group>
        ),
      });
    } catch (e) {
      notifications.show({
        color: "red",
        message: problem(t, "share.debugFailed", { message: String(e) }),
      });
    }
  }
  return (
    <Menu.Item onClick={() => void save()}>{t("share.debugMenu")}</Menu.Item>
  );
}

type Format = "html" | "markdown" | "pdf";

/** Exports the whole journal: as one web page laid out like the app (pictures inside), as
 * Markdown, or as a PDF through the system print dialog. */
export function ExportDialog({
  characterId,
  opened,
  onClose,
  onPrint,
}: {
  characterId: number;
  opened: boolean;
  onClose: () => void;
  /** Opens the journal laid out for paper, where the print dialog saves a PDF. */
  onPrint: () => void;
}) {
  const { t } = useTranslation();
  const [format, setFormat] = useState<Format>("html");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!opened) return;
    setDone(null);
    setError(null);
    void exportFolder().then((f) => setFolder((current) => current || f));
  }, [opened]);

  async function save() {
    if (format === "pdf") {
      onPrint();
      return;
    }
    setBusy(true);
    setError(null);
    setDone(null);
    try {
      const page =
        format === "html"
          ? exportPage(await loadExportData(characterId, true))
          : null;
      setDone(
        await exportCharacter(
          characterId,
          folder,
          format === "html" ? "html" : "markdown",
          page,
        ),
      );
      rememberExportFolder(folder);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal opened={opened} onClose={onClose} title={t("share.exportTitle")}>
      <Stack>
        <Radio.Group
          label={t("share.format")}
          value={format}
          onChange={(v) => {
            setFormat(v as Format);
            setDone(null);
            setError(null);
          }}
        >
          <Stack gap="xs" mt="xs">
            <Radio value="html" label={t("share.formats.html")} />
            <Radio value="pdf" label={t("share.formats.pdf")} />
            <Radio value="markdown" label={t("share.formats.markdown")} />
          </Stack>
        </Radio.Group>
        <Text size="sm">{t(`share.formatHelp.${format}`)}</Text>
        {format !== "pdf" && (
          <FolderField
            label={t("share.folder")}
            value={folder}
            onChange={setFolder}
          />
        )}
        {error && (
          <Alert color="red">
            {problem(t, "share.exportFailed", { message: error })}
          </Alert>
        )}
        {done && (
          <Alert color="green">
            <Stack gap="xs">
              <Text size="sm">{t("share.exported", { path: done })}</Text>
              <Button
                size="xs"
                variant="light"
                color="green"
                w="fit-content"
                onClick={() => void showInFolder(done)}
              >
                {t("share.showInFolder")}
              </Button>
            </Stack>
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {t("share.close")}
          </Button>
          <Button
            loading={busy}
            disabled={format !== "pdf" && folder.trim() === ""}
            onClick={() => void save()}
          >
            {format === "pdf" ? t("share.print") : t("share.export")}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
