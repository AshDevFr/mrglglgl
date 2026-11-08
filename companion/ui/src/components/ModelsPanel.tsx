import {
  Alert,
  Badge,
  Button,
  Group,
  Modal,
  Progress,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  cancelDownload,
  deleteModel,
  downloadModel,
  models,
  onModelDownload,
} from "@/api/commands";
import type { DownloadProgress } from "@/bindings/DownloadProgress";
import type { ModelView } from "@/bindings/ModelView";
import { asFailure, failureProblem } from "@/i18n/failure";
import { problem } from "@/i18n/problems";

/** Gigabytes with two decimals, as model sizes are told. */
/** Models asking for more memory than this are flagged: slower, and too much for many machines. */
const BIG_MODEL_RAM_GB = 8;

export function gigabytes(bytes: number): string {
  return (bytes / 1e9).toFixed(2);
}

interface Props {
  /** Makes a model the one writing uses. */
  onChoose: (modelId: string) => void;
}

/** The writing models: which one is used, downloads with a size confirmation, progress and a
 * working stop, and the first-run advice for this computer. */
export function ModelsPanel({ onChoose }: Props) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const view = useQuery({ queryKey: ["models"], queryFn: models });
  const [showAll, setShowAll] = useState(false);
  const [confirming, setConfirming] = useState<ModelView | null>(null);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    onModelDownload(setProgress).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  async function download(model: ModelView) {
    setConfirming(null);
    setError(null);
    setProgress({
      modelId: model.id,
      downloaded: model.partialBytes,
      total: model.sizeBytes,
    });
    try {
      await downloadModel(model.id);
    } catch (e) {
      // A stop the player asked for is not an error to show.
      if (asFailure(e).kind !== "cancelled") setError(e);
    } finally {
      setProgress(null);
      await queryClient.invalidateQueries({ queryKey: ["models"] });
      await queryClient.invalidateQueries({ queryKey: ["writing"] });
    }
  }

  async function remove(model: ModelView) {
    await deleteModel(model.id);
    await queryClient.invalidateQueries({ queryKey: ["models"] });
  }

  if (!view.data) return null;
  const { advice, chosenId } = view.data;
  // Once a model is downloaded and in use, the others are folded away until asked for; a
  // download in progress keeps every row in sight.
  const inUse = view.data.models.find((m) => m.installed && m.id === chosenId);
  const folded = inUse !== undefined && !showAll && progress === null;
  const shown = folded ? [inUse] : view.data.models;
  const anyInstalled = view.data.models.some((m) => m.installed);
  const suggested =
    advice.kind === "recommend"
      ? view.data.models.find((m) => m.id === advice.id)
      : undefined;

  return (
    <Stack>
      <Title order={2} size="h3">
        {t("models.title")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("models.help")}
      </Text>
      {!anyInstalled && suggested && !progress && (
        <Alert color="teal">
          <Stack gap="xs">
            <Text size="sm">
              {t("models.firstRun", { name: suggested.name })}
            </Text>
            <Button w="fit-content" onClick={() => setConfirming(suggested)}>
              {t("models.firstRunAction", {
                name: suggested.name,
                size: gigabytes(suggested.sizeBytes),
              })}
            </Button>
          </Stack>
        </Alert>
      )}
      {advice.kind === "notEnoughMemory" && (
        <Alert color="yellow">
          {problem(t, "models.notEnoughMemory", {
            has: advice.hasGb,
            needed: advice.neededGb,
          })}
        </Alert>
      )}
      {advice.kind === "notEnoughDisk" && (
        <Alert color="yellow">
          {problem(t, "models.notEnoughDisk", {
            free: advice.freeGb,
            needed: advice.neededGb,
          })}
        </Alert>
      )}
      {error !== null && (
        <Alert color="red">{failureProblem(t, "models.failed", error)}</Alert>
      )}

      {shown.map((m) => {
        const downloading = progress?.modelId === m.id;
        return (
          <Stack key={m.id} gap={4} className="model-row">
            <Group gap="xs">
              <Text fw={600}>{m.name}</Text>
              {m.recommended && (
                <Badge variant="light" color="teal">
                  {t("models.recommended")}
                </Badge>
              )}
              {m.installed && m.id === chosenId && (
                <Badge variant="outline">{t("models.inUse")}</Badge>
              )}
            </Group>
            <Text size="sm" c="dimmed">
              {t("models.size", { size: gigabytes(m.sizeBytes) })},{" "}
              {t("models.licence", { licence: m.licence })}
            </Text>
            {m.proseNote && <Text size="sm">{m.proseNote}</Text>}
            {m.minRamGb > BIG_MODEL_RAM_GB && (
              <Alert color="yellow" variant="light" p="xs">
                {t("models.big", { gb: m.minRamGb })}
              </Alert>
            )}
            {downloading && progress && (
              <Stack gap={2}>
                <Progress
                  value={
                    (progress.downloaded / Math.max(progress.total, 1)) * 100
                  }
                  aria-label={t("models.progress", {
                    done: gigabytes(progress.downloaded),
                    total: gigabytes(progress.total),
                  })}
                />
                <Text size="xs">
                  {t("models.progress", {
                    done: gigabytes(progress.downloaded),
                    total: gigabytes(progress.total),
                  })}
                </Text>
              </Stack>
            )}
            <Group gap="xs">
              {downloading ? (
                <Button
                  size="xs"
                  variant="light"
                  color="gray"
                  onClick={() => void cancelDownload()}
                >
                  {t("models.cancelDownload")}
                </Button>
              ) : m.installed ? (
                <>
                  {m.id !== chosenId && (
                    <Button
                      size="xs"
                      variant="light"
                      onClick={() => onChoose(m.id)}
                    >
                      {t("models.use")}
                    </Button>
                  )}
                  <Button
                    size="xs"
                    variant="subtle"
                    color="red"
                    onClick={() => void remove(m)}
                  >
                    {t("models.delete")}
                  </Button>
                </>
              ) : (
                <Button
                  size="xs"
                  variant="light"
                  disabled={progress !== null}
                  onClick={() => setConfirming(m)}
                >
                  {m.partialBytes > 0
                    ? t("models.resume")
                    : t("models.download")}
                </Button>
              )}
            </Group>
          </Stack>
        );
      })}
      {inUse !== undefined && progress === null && (
        <Button
          variant="subtle"
          size="xs"
          w="fit-content"
          onClick={() => setShowAll((v) => !v)}
        >
          {showAll
            ? t("models.showInUse")
            : t("models.showAll", { count: view.data.models.length })}
        </Button>
      )}

      <Modal
        opened={confirming !== null}
        onClose={() => setConfirming(null)}
        title={
          confirming ? t("models.confirmTitle", { name: confirming.name }) : ""
        }
      >
        {confirming && (
          <Stack>
            <Text size="sm">
              {t("models.confirmBody", {
                size: gigabytes(confirming.sizeBytes),
              })}
            </Text>
            <Group justify="flex-end">
              <Button variant="default" onClick={() => setConfirming(null)}>
                {t("models.cancel")}
              </Button>
              <Button onClick={() => void download(confirming)}>
                {t("models.confirm", { size: gigabytes(confirming.sizeBytes) })}
              </Button>
            </Group>
          </Stack>
        )}
      </Modal>
    </Stack>
  );
}
