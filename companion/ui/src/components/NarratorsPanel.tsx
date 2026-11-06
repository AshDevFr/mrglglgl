import {
  Alert,
  Button,
  Group,
  Modal,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  copyNarrator,
  defaultExportFolder,
  deleteNarrator,
  exportNarrator,
  importNarrator,
  listNarrators,
  saveNarrator,
} from "@/api/commands";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { NarratorSaved } from "@/bindings/NarratorSaved";
import { problem } from "@/i18n/problems";

/** Every narrator: built-ins to copy, the player's own to edit, share or remove. */
export function NarratorsPanel({ onEdit }: { onEdit: (id: string) => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const narrators = useQuery({
    queryKey: ["narrators"],
    queryFn: listNarrators,
  });
  const [exporting, setExporting] = useState<NarratorInfo | null>(null);
  const [removing, setRemoving] = useState<NarratorInfo | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["narrators"] });
  const attempt = async (work: () => Promise<void>) => {
    setFailure(null);
    setMessage(null);
    try {
      await work();
    } catch (e) {
      setFailure(String(e));
    }
  };
  const settle = async (result: NarratorSaved, then: (id: string) => void) => {
    if (result.kind === "saved") {
      await refresh();
      then(result.narrator.id);
    } else {
      setFailure(
        result.problems.map((p) => `${p.field}: ${p.problem}`).join(", "),
      );
    }
  };

  const copy = (id: string) =>
    attempt(async () => {
      await settle(await saveNarrator(await copyNarrator(id)), onEdit);
    });
  const importFile = (file: File) =>
    attempt(async () => {
      const result = await importNarrator(await file.text());
      await settle(result, () => {
        if (result.kind === "saved")
          setMessage(t("narrators.imported", { name: result.narrator.name }));
      });
    });

  const all = narrators.data ?? [];
  const row = (n: NarratorInfo) => (
    <li key={n.id} aria-label={n.name} style={{ listStyle: "none" }}>
      <Group justify="space-between" wrap="nowrap" py={4}>
        <Text>{n.name}</Text>
        <Group gap="xs" wrap="nowrap">
          {n.custom ? (
            <>
              <Button size="xs" variant="light" onClick={() => onEdit(n.id)}>
                {t("narrators.edit")}
              </Button>
              <Button
                size="xs"
                variant="subtle"
                onClick={() => setExporting(n)}
              >
                {t("narrators.export")}
              </Button>
              <Button
                size="xs"
                variant="subtle"
                color="red"
                onClick={() => setRemoving(n)}
              >
                {t("narrators.delete")}
              </Button>
            </>
          ) : (
            <Button size="xs" variant="subtle" onClick={() => void copy(n.id)}>
              {t("narrators.editCopy")}
            </Button>
          )}
        </Group>
      </Group>
    </li>
  );

  return (
    <Stack gap="sm">
      <Title order={2} size="h3">
        {t("narrators.title")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("narrators.help")}
      </Text>
      <Text size="sm" fw={600}>
        {t("narrators.builtIn")}
      </Text>
      <ul style={{ margin: 0, padding: 0 }}>
        {all.filter((n) => !n.custom).map(row)}
      </ul>
      {all.some((n) => n.custom) && (
        <>
          <Text size="sm" fw={600}>
            {t("narrators.mine")}
          </Text>
          <ul style={{ margin: 0, padding: 0 }}>
            {all.filter((n) => n.custom).map(row)}
          </ul>
        </>
      )}
      <label>
        <Text size="sm">{t("narrators.import")}</Text>
        <Text size="xs" c="dimmed">
          {t("narrators.importHelp")}
        </Text>
        <input
          type="file"
          accept=".toml,text/plain"
          aria-label={t("narrators.import")}
          onChange={(e) => {
            const file = e.currentTarget.files?.[0];
            e.currentTarget.value = "";
            if (file) void importFile(file);
          }}
        />
      </label>
      {message && <Alert color="green">{message}</Alert>}
      {failure && (
        <Alert color="red">
          {problem(t, "narrators.fileFailed", { message: failure })}
        </Alert>
      )}
      <ExportDialog narrator={exporting} onClose={() => setExporting(null)} />
      <Modal
        opened={removing !== null}
        onClose={() => setRemoving(null)}
        title={t("narrators.deleteTitle", { name: removing?.name ?? "" })}
      >
        <Stack>
          <Text size="sm">{t("narrators.deleteBody")}</Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setRemoving(null)}>
              {t("narrators.cancel")}
            </Button>
            <Button
              color="red"
              onClick={() => {
                const id = removing?.id;
                setRemoving(null);
                if (id)
                  void attempt(async () => {
                    await deleteNarrator(id);
                    await refresh();
                  });
              }}
            >
              {t("narrators.deleteConfirm")}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}

function ExportDialog({
  narrator,
  onClose,
}: {
  narrator: NarratorInfo | null;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [folder, setFolder] = useState("");
  const [path, setPath] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const opened = narrator !== null;

  useEffect(() => {
    if (!opened) return;
    setPath(null);
    setFailure(null);
    void defaultExportFolder().then((f) =>
      setFolder((current) => current || f || ""),
    );
  }, [opened]);

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={t("narrators.exportTitle", { name: narrator?.name ?? "" })}
    >
      <Stack>
        <Text size="sm">{t("narrators.exportHelp")}</Text>
        <TextInput
          label={t("narrators.folder")}
          value={folder}
          onChange={(e) => setFolder(e.currentTarget.value)}
        />
        {path && (
          <Alert color="green">{t("narrators.exported", { path })}</Alert>
        )}
        {failure && (
          <Alert color="red">
            {problem(t, "narrators.fileFailed", { message: failure })}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {t("narrators.close")}
          </Button>
          <Button
            disabled={folder.trim() === ""}
            onClick={() => {
              if (!narrator) return;
              setFailure(null);
              exportNarrator(narrator.id, folder)
                .then(setPath)
                .catch((e) => setFailure(String(e)));
            }}
          >
            {t("narrators.export")}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
