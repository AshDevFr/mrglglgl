import { Alert, Button, Group, Stack, Text } from "@mantine/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import type { WowRoot } from "@/bindings/WowRoot";
import { problem } from "@/i18n/problems";

interface Props {
  /** The folder the player chose, as edited here; empty means find it automatically. */
  value: string;
  /** The folder chosen as saved, so an unsaved change can say so. */
  saved: string | null;
  /** The installs in use with the saved settings. */
  roots: WowRoot[];
  onChange: (folder: string) => void;
}

/**
 * The game folder: what is in use and where it came from. A found folder is never written into
 * the settings, so the app keeps finding the game if it moves; only a folder the player picks is
 * kept, until they ask to find it automatically again.
 */
export function WowFolderField({ value, saved, roots, onChange }: Props) {
  const { t } = useTranslation();
  const chosen = value.trim();
  const unsaved = chosen !== (saved ?? "");

  async function choose() {
    const picked = await open({
      directory: true,
      multiple: false,
      defaultPath: chosen || roots[0]?.path || undefined,
      title: t("settings.wowFolder"),
    });
    if (typeof picked === "string") onChange(picked);
  }

  return (
    <Stack gap={6}>
      <Text size="sm" fw={500}>
        {t("settings.wowFolder")}
      </Text>
      {chosen ? (
        <Text size="sm">
          {t("settings.wowChosen")} <strong>{chosen}</strong>
        </Text>
      ) : roots.length > 0 ? (
        roots.map((root) => (
          <Text size="sm" key={root.path}>
            {t("settings.wowFound")} <strong>{root.path}</strong>
          </Text>
        ))
      ) : (
        <Alert color="yellow">{problem(t, "settings.foundNone")}</Alert>
      )}
      {chosen && !unsaved && roots.length === 0 && (
        <Alert color="yellow">{problem(t, "settings.chosenMissing")}</Alert>
      )}
      {unsaved && (
        <Text size="xs" c="dimmed">
          {t("settings.wowUnsaved")}
        </Text>
      )}
      <Group gap="xs">
        <Button size="xs" variant="default" onClick={() => void choose()}>
          {chosen || roots.length === 0
            ? t("settings.wowChoose")
            : t("settings.wowChooseAnother")}
        </Button>
        {chosen && (
          <Button size="xs" variant="subtle" onClick={() => onChange("")}>
            {t("settings.wowAutomatic")}
          </Button>
        )}
      </Group>
      <Text size="xs" c="dimmed">
        {t("settings.wowFolderHelp")}
      </Text>
    </Stack>
  );
}
