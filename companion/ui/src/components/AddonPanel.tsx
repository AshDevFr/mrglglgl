import {
  Alert,
  Badge,
  Button,
  Group,
  Stack,
  Switch,
  Text,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  addonRelease,
  addonViews,
  installAddon,
  removeAddon,
  restoreAddon,
  setAddonPrereleases,
} from "@/api/commands";
import type { AddonProblem } from "@/bindings/AddonProblem";
import type { AddonState } from "@/bindings/AddonState";
import type { AddonView } from "@/bindings/AddonView";
import type { ReleaseCheck } from "@/bindings/ReleaseCheck";
import { failureProblem } from "@/i18n/failure";
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

interface RowProps {
  view: AddonView;
  check: ReleaseCheck | undefined;
  onChanged: () => Promise<void>;
}

function FlavorRow({ view, check, onChanged }: RowProps) {
  const { t, i18n } = useTranslation();
  const addon = view.status;
  const [busy, setBusy] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [done, setDone] = useState<string | null>(null);

  const release = check?.release ?? null;
  const foreign = view.ownership === "foreign";
  const ours = view.ownership === "managed" || view.ownership === "changed";
  const update = check?.updates.includes(view.flavorPath) ?? false;

  async function act(name: string, run: () => Promise<string>) {
    setBusy(name);
    setError(null);
    setDone(null);
    try {
      setDone(await run());
      setConfirming(false);
      await onChanged();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(null);
    }
  }

  const install = (takeOver: boolean) =>
    act("install", async () => {
      const result = await installAddon(view.flavorPath, takeOver);
      return t("addon.done.installed", { version: result.version ?? "" });
    });

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
      {view.ownership !== "notInstalled" && view.ownership !== "managed" && (
        <Text size="xs" c="dimmed">
          {t(`addon.ownership.${view.ownership}`)}
        </Text>
      )}
      {addon.problems.map((p) => (
        <Alert key={p.kind} color={p.kind === "errors" ? "orange" : "yellow"}>
          {describeProblem(t, i18n.language, p)}
        </Alert>
      ))}
      <Group gap="xs" mt={4}>
        {release && view.ownership === "notInstalled" && (
          <Button
            size="xs"
            loading={busy === "install"}
            disabled={busy !== null}
            onClick={() => void install(false)}
          >
            {t("addon.install", { version: release.version })}
          </Button>
        )}
        {release && ours && update && (
          <Button
            size="xs"
            loading={busy === "install"}
            disabled={busy !== null}
            onClick={() => void install(false)}
          >
            {t("addon.update", { version: release.version })}
          </Button>
        )}
        {release && foreign && (
          <Button
            size="xs"
            variant="default"
            loading={busy === "install"}
            disabled={busy !== null}
            onClick={() => void install(true)}
          >
            {t("addon.takeOver", { version: release.version })}
          </Button>
        )}
        {view.rollback && view.ownership !== "development" && (
          <Button
            size="xs"
            variant="default"
            loading={busy === "restore"}
            disabled={busy !== null}
            onClick={() =>
              void act("restore", async () => {
                const result = await restoreAddon(view.flavorPath);
                return t("addon.done.installed", {
                  version: result.version ?? "",
                });
              })
            }
          >
            {view.rollback.version
              ? t("addon.restore", { version: view.rollback.version })
              : t("addon.restoreUnknown")}
          </Button>
        )}
        {ours && !confirming && (
          <Button
            size="xs"
            variant="subtle"
            color="red"
            disabled={busy !== null}
            onClick={() => setConfirming(true)}
          >
            {t("addon.remove")}
          </Button>
        )}
      </Group>
      {confirming && (
        <Alert
          color="red"
          title={t("addon.removeTitle", { flavor: addon.flavor })}
        >
          <Stack gap="xs">
            <Text size="sm">{t("addon.removeExplain")}</Text>
            <Group gap="xs">
              <Button
                size="xs"
                color="red"
                loading={busy === "remove"}
                onClick={() =>
                  void act("remove", async () => {
                    await removeAddon(view.flavorPath, false);
                    return t("addon.done.removed");
                  })
                }
              >
                {t("addon.removeConfirm")}
              </Button>
              <Button
                size="xs"
                variant="default"
                disabled={busy !== null}
                onClick={() => setConfirming(false)}
              >
                {t("addon.cancel")}
              </Button>
            </Group>
          </Stack>
        </Alert>
      )}
      {done && <Alert color="green">{done}</Alert>}
      {error !== null && (
        <Alert color="red">
          {failureProblem(t, "addon.actionFailed", error)}
        </Alert>
      )}
    </Stack>
  );
}

/** The addon in each WoW: Forever folder found: its state, and installing, updating or removing it. */
export function AddonPanel() {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const views = useQuery({ queryKey: ["addonViews"], queryFn: addonViews });
  const release = useQuery({
    queryKey: ["addonRelease"],
    queryFn: () => addonRelease(false),
    retry: false,
  });
  const [checking, setChecking] = useState(false);

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: ["addonViews"] });
    await queryClient.invalidateQueries({ queryKey: ["addonRelease"] });
  }

  async function checkAgain() {
    setChecking(true);
    try {
      queryClient.setQueryData(["addonRelease"], await addonRelease(true));
    } catch {
      await release.refetch();
    } finally {
      setChecking(false);
    }
  }

  async function choosePrereleases(on: boolean) {
    await setAddonPrereleases(on);
    await refresh();
  }

  const latest = release.data?.release;
  return (
    <Stack>
      <Title order={2} size="h3">
        {t("addon.title")}
      </Title>
      {views.isError && (
        <Alert color="red">
          {problem(t, "errors.load", { message: String(views.error) })}
        </Alert>
      )}
      {release.isError && (
        <Alert color="yellow">
          {failureProblem(t, "addon.releaseFailed", release.error)}
        </Alert>
      )}
      {release.data?.stale && (
        <Alert color="yellow">{problem(t, "addon.staleListing")}</Alert>
      )}
      {release.data && (
        <Group gap="xs">
          <Text size="sm">
            {latest
              ? t("addon.latest", {
                  version: latest.version,
                  when: when(i18n.language, release.data.checked),
                })
              : release.data.testVersion
                ? t("addon.onlyTestVersion", {
                    version: release.data.testVersion,
                  })
                : t("addon.noRelease")}
          </Text>
          <Button
            size="xs"
            variant="subtle"
            loading={checking}
            onClick={() => void checkAgain()}
          >
            {t("addon.checkAgain")}
          </Button>
        </Group>
      )}
      <Switch
        label={t("addon.prereleases")}
        description={t("addon.prereleasesHelp")}
        checked={release.data?.prereleases ?? false}
        disabled={release.isLoading}
        onChange={(e) => void choosePrereleases(e.currentTarget.checked)}
      />
      {views.data?.length === 0 && <Text size="sm">{t("addon.noFlavor")}</Text>}
      {views.data?.map((view) => (
        <FlavorRow
          key={view.flavorPath}
          view={view}
          check={release.data}
          onChanged={refresh}
        />
      ))}
    </Stack>
  );
}
