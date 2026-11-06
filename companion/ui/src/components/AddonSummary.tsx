import { Alert, Button, Stack, Text } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { addonViews } from "@/api/commands";
import type { AddonView } from "@/bindings/AddonView";
import { formatTime } from "@/lib/format";

/** The addon at a glance on the home page: a quiet line when it is recording, a warning when it
 * is missing or needs attention, with the way to the Addon settings. */
export function AddonSummary({ onOpen }: { onOpen: () => void }) {
  const { t, i18n } = useTranslation();
  const views = useQuery({ queryKey: ["addonViews"], queryFn: addonViews });
  if (!views.data || views.data.length === 0) return null;

  const line = (view: AddonView) => {
    const s = view.status;
    const version = s.installed?.development
      ? t("addon.development")
      : s.installed?.version
        ? t("addon.version", { version: s.installed.version })
        : null;
    const last = s.lastRun
      ? t("home.addonLastNoted", {
          character: s.lastRun.character,
          when: formatTime(s.lastRun.login, i18n.language),
        })
      : null;
    return [
      views.data.length > 1 ? s.flavor : null,
      version,
      t(`addon.state.${s.state}`),
      last,
    ]
      .filter(Boolean)
      .join(" · ");
  };

  const missing = views.data.filter((v) => v.status.state === "notInstalled");
  const troubled = views.data.filter(
    (v) => v.status.state !== "notInstalled" && v.status.problems.length > 0,
  );
  const quiet = views.data.filter((v) => v.status.state === "quiet");

  return (
    <Stack gap="xs">
      {missing.length > 0 && (
        <Alert color="yellow" title={t("home.addonMissingTitle")}>
          <Stack gap="xs">
            <Text size="sm">{t("home.addonMissing")}</Text>
            <Button size="xs" w="fit-content" onClick={onOpen}>
              {t("home.addonInstall")}
            </Button>
          </Stack>
        </Alert>
      )}
      {(troubled.length > 0 || quiet.length > 0) && (
        <Alert color="yellow">
          <Stack gap="xs">
            <Text size="sm">{t("home.addonAttention")}</Text>
            <Button size="xs" variant="light" w="fit-content" onClick={onOpen}>
              {t("home.addonOpen")}
            </Button>
          </Stack>
        </Alert>
      )}
      {views.data
        .filter((v) => v.status.state !== "notInstalled")
        .map((v) => (
          <Text key={v.flavorPath} size="sm" c="dimmed">
            {t("home.addonLine", { line: line(v) })}
          </Text>
        ))}
    </Stack>
  );
}
