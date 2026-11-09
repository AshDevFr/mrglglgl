import { Button, Group, Stack, Text, Title } from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { getSettings, models, saveSettings } from "@/api/commands";
import { problem } from "@/i18n/problems";
import { FolderField } from "./FolderField";
import { gigabytes } from "./ModelsPanel";

/**
 * The first steps, in order: the game folder (found, or asked for), how journals arrive, and a
 * writing model, offered but never downloaded without the player choosing it in Settings.
 */
export function FirstRun({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const view = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const catalogue = useQuery({ queryKey: ["models"], queryFn: models });
  const [folder, setFolder] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function saveFolder() {
    if (!view.data) return;
    setError(null);
    try {
      await saveSettings({
        ...view.data.settings,
        wowRootOverride: folder.trim(),
      });
      await queryClient.invalidateQueries();
    } catch (e) {
      setError(String(e));
    }
  }

  const roots = view.data?.roots ?? [];
  const installed = catalogue.data?.models.some((m) => m.installed) ?? false;
  const advice = catalogue.data?.advice;
  const offered =
    advice?.kind === "recommend"
      ? catalogue.data?.models.find((m) => m.id === advice.id)
      : undefined;
  const local = view.data?.settings.writing.provider !== "endpoint";

  return (
    <Stack gap="lg" className="first-run">
      <Title order={2} size="h3">
        {t("firstRun.title")}
      </Title>

      <Stack gap="xs">
        {view.data &&
          (roots.length > 0 ? (
            <Text>{t("firstRun.folderFound", { path: roots[0].path })}</Text>
          ) : (
            <>
              <Text>{problem(t, "settings.foundNone")}</Text>
              <Group align="flex-end" gap="xs">
                <div style={{ width: 420 }}>
                  <FolderField
                    label={t("firstRun.folderLabel")}
                    value={folder}
                    onChange={setFolder}
                  />
                </div>
                <Button
                  variant="light"
                  disabled={folder.trim() === ""}
                  onClick={() => void saveFolder()}
                >
                  {t("firstRun.saveFolder")}
                </Button>
              </Group>
              {error && (
                <Text size="sm" c="red" role="status">
                  {problem(t, "settings.error", { message: error })}
                </Text>
              )}
            </>
          ))}
      </Stack>

      <Stack gap="xs">
        <Text fw={600}>{t("firstRun.syncTitle")}</Text>
        <Text>{t("sync.explain")}</Text>
      </Stack>

      {local && catalogue.data && (
        <Stack gap="xs">
          <Text fw={600}>{t("firstRun.modelTitle")}</Text>
          {installed ? (
            <Text>{t("firstRun.modelReady")}</Text>
          ) : advice?.kind === "notEnoughMemory" ? (
            <Text>
              {problem(t, "models.notEnoughMemory", {
                has: advice.hasGb,
                needed: advice.neededGb,
              })}
            </Text>
          ) : advice?.kind === "notEnoughDisk" ? (
            <Text>
              {problem(t, "models.notEnoughDisk", {
                free: advice.freeGb,
                needed: advice.neededGb,
              })}
            </Text>
          ) : (
            offered && (
              <Text>
                {t("firstRun.modelOffer", {
                  name: offered.name,
                  size: gigabytes(offered.sizeBytes),
                })}
              </Text>
            )
          )}
          {!installed && (
            <Button variant="light" w="fit-content" onClick={onOpenSettings}>
              {t("firstRun.chooseModel")}
            </Button>
          )}
        </Stack>
      )}

      {/* Writing through an endpoint: ready once its address is set. */}
      {!local && view.data && (
        <Stack gap="xs">
          <Text fw={600}>{t("firstRun.modelTitle")}</Text>
          {view.data.settings.writing.endpointUrl ? (
            <Text>{t("firstRun.modelReady")}</Text>
          ) : (
            <>
              <Text>{problem(t, "writing.blocked.noEndpoint")}</Text>
              <Button variant="light" w="fit-content" onClick={onOpenSettings}>
                {t("firstRun.setUpEndpoint")}
              </Button>
            </>
          )}
        </Stack>
      )}
    </Stack>
  );
}
