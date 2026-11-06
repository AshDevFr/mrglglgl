import { Alert, Badge, Group, Stack, Text, Title } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { addonStatus } from "@/api/commands";
import type { AddonProblem } from "@/bindings/AddonProblem";
import type { AddonState } from "@/bindings/AddonState";
import type { FlavorAddon } from "@/bindings/FlavorAddon";
import { problem } from "@/i18n/problems";

const COLORS: Record<AddonState, string> = {
  notInstalled: "gray",
  waiting: "blue",
  recording: "teal",
  quiet: "yellow",
};

function when(lang: string, unix: number): string {
  return new Intl.DateTimeFormat(lang, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(unix * 1000));
}

function describeProblem(
  t: TFunction,
  lang: string,
  found: AddonProblem,
): string {
  switch (found.kind) {
    case "newerSchema":
      return problem(t, "addon.problem.newerSchema");
    case "updatePending":
      return problem(t, "addon.problem.updatePending", {
        running: found.running,
        installed: found.installed,
      });
    case "outOfDate":
      return problem(t, "addon.problem.outOfDate");
    case "errors":
      return problem(t, "addon.problem.errors", {
        message: found.message ?? "",
        when: found.time === null ? "" : when(lang, found.time),
      });
  }
}

function FlavorRow({ addon }: { addon: FlavorAddon }) {
  const { t, i18n } = useTranslation();
  const version = addon.installed?.development
    ? t("addon.development")
    : addon.installed?.version
      ? t("addon.version", { version: addon.installed.version })
      : null;
  return (
    <Stack gap={4}>
      <Group gap="xs">
        <Text fw={600}>{addon.flavor}</Text>
        <Badge color={COLORS[addon.state]} variant="light">
          {t(`addon.state.${addon.state}`)}
        </Badge>
        {version && (
          <Text size="sm" c="dimmed">
            {version}
          </Text>
        )}
      </Group>
      <Text size="sm">
        {addon.state === "recording" || addon.state === "quiet"
          ? t(`addon.next.${addon.state}`, {
              character: addon.lastRun?.character ?? "",
              when: when(i18n.language, addon.lastRun?.login ?? 0),
            })
          : t(`addon.next.${addon.state}`, { folder: addon.folder })}
      </Text>
      {addon.clientVersion && (
        <Text size="xs" c="dimmed">
          {t("addon.client", { version: addon.clientVersion })}
        </Text>
      )}
      {addon.problems.map((p) => (
        <Alert key={p.kind} color={p.kind === "errors" ? "orange" : "yellow"}>
          {describeProblem(t, i18n.language, p)}
        </Alert>
      ))}
    </Stack>
  );
}

/** Whether the addon is installed in each WoW: Forever folder found, and recording. */
export function AddonPanel() {
  const { t } = useTranslation();
  const status = useQuery({ queryKey: ["addonStatus"], queryFn: addonStatus });
  return (
    <Stack>
      <Title order={2} size="h3">
        {t("addon.title")}
      </Title>
      {status.isError && (
        <Alert color="red">
          {problem(t, "errors.load", { message: String(status.error) })}
        </Alert>
      )}
      {status.data?.length === 0 && (
        <Text size="sm">{t("addon.noFlavor")}</Text>
      )}
      {status.data?.map((addon) => (
        <FlavorRow key={addon.folder} addon={addon} />
      ))}
    </Stack>
  );
}
