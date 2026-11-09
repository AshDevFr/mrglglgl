import {
  Alert,
  Anchor,
  Button,
  Drawer,
  Group,
  Loader,
  NavLink,
  Progress,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  activity,
  cancelDownload,
  cancelPreview,
  cancelWriting,
  listCharacters,
  models,
  onWriting,
  retryWriting,
  setWritingPaused,
} from "@/api/commands";
import type { Activity } from "@/bindings/Activity";
import type { Job } from "@/bindings/Job";
import { type Problem, problem } from "@/i18n/problems";
import { formatDay } from "@/lib/format";
import { displayName } from "@/lib/journal";
import { useSiteBuilding } from "@/lib/siteBuild";
import { slot } from "@/lib/writing";

/** How often the list is read again while something runs, and while nothing does. */
const BUSY_POLL_MS = 2000;
const IDLE_POLL_MS = 15000;

/** Things running or waiting their turn; failed entries wait for the player, not the app. */
function busyCount(now: Activity | undefined, building: boolean): number {
  if (!now) return building ? 1 : 0;
  const jobs = now.jobs.filter((j) => j.state !== "failed").length;
  return (
    jobs + (now.preview ? 1 : 0) + (now.download ? 1 : 0) + (building ? 1 : 0)
  );
}

/** The text of the entry being written, as it streams, to count its words. */
function useStreamingText(): { key: string | null; text: string } {
  const [live, setLive] = useState<{ key: string | null; text: string }>({
    key: null,
    text: "",
  });
  const queryClient = useQueryClient();
  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    onWriting((event) => {
      const key = `${event.characterId}|${slot(event.kind, event.startDay)}`;
      if (event.type === "chunk")
        setLive((l) =>
          l.key === key
            ? { key, text: l.text + event.text }
            : { key, text: event.text },
        );
      else if (event.type === "started" || event.type === "restarted")
        setLive({ key, text: "" });
      else setLive({ key: null, text: "" });
      if (event.type === "started" || event.type === "finished")
        void queryClient.invalidateQueries({ queryKey: ["activity"] });
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [queryClient]);
  return live;
}

/** An entry in the sidebar showing whether the app is working, and a drawer listing
 * everything it works on, for every character: the entry being written, the line of entries
 * waiting, those that failed, a narrator preview, a model download and a website build. */
export function ActivityPanel() {
  const { t } = useTranslation();
  const [opened, setOpened] = useState(false);
  const building = useSiteBuilding();
  const now = useQuery({
    queryKey: ["activity"],
    queryFn: activity,
    refetchInterval: (query) =>
      busyCount(query.state.data, building) > 0 ? BUSY_POLL_MS : IDLE_POLL_MS,
  });
  const count = busyCount(now.data, building);
  // Entries wait but none can be written: say so rather than spin as if working.
  const stuck =
    !!now.data?.blocked &&
    !now.data.jobs.some((j) => j.state === "running") &&
    !now.data.preview &&
    !now.data.download &&
    !building;
  return (
    <>
      <NavLink
        component="button"
        label={count > 0 ? t("activity.count", { count }) : t("activity.title")}
        leftSection={
          stuck ? (
            <Text
              component="span"
              c="yellow"
              fw={700}
              aria-label={t("activity.blocked")}
            >
              !
            </Text>
          ) : count > 0 ? (
            <Loader size={12} />
          ) : undefined
        }
        onClick={() => setOpened(true)}
      />
      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        position="right"
        title={t("activity.title")}
      >
        {now.data && (
          <ActivityList
            now={now.data}
            building={building}
            onLeave={() => setOpened(false)}
          />
        )}
      </Drawer>
    </>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <Stack gap={6}>
      <Title order={4} size="h6">
        {title}
      </Title>
      <ul className="activity-list">{children}</ul>
    </Stack>
  );
}

function ActivityList({
  now,
  building,
  onLeave,
}: {
  now: Activity;
  building: boolean;
  /** Closes the list before going to a page from it. */
  onLeave: () => void;
}) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const live = useStreamingText();
  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const modelList = useQuery({
    queryKey: ["models"],
    queryFn: models,
    enabled: now.download !== null,
  });
  const refresh = () =>
    Promise.all(
      [["activity"], ["writing"], ["review"], ["settings"]].map((queryKey) =>
        queryClient.invalidateQueries({ queryKey }),
      ),
    );

  const name = (id: number) => {
    const found = characters.data?.find((c) => c.character.id === id);
    return found ? displayName(found.character) : "";
  };
  // A task opens where it lives: an entry on its day in the journal, the rest in Settings.
  const openDay = (job: Job) => {
    onLeave();
    void navigate({
      to: "/characters/$characterId",
      params: { characterId: String(job.characterId) },
      search: { day: job.startDay },
    });
  };
  const openSettings = () => {
    onLeave();
    void navigate({ to: "/settings" });
  };
  const label = (job: Job) => {
    const date = formatDay(job.startDay, i18n.language, true);
    const when = job.kind === "week" ? t("writing.chapter", { date }) : date;
    return `${name(job.characterId)} · ${when}`;
  };

  const running = now.jobs.filter((j) => j.state === "running");
  const waiting = now.jobs.filter((j) => j.state === "pending");
  const failed = now.jobs.filter((j) => j.state === "failed");
  const nothing =
    running.length + waiting.length + failed.length === 0 &&
    !now.preview &&
    !now.download &&
    !building;
  const words = live.text.split(/\s+/).filter(Boolean).length;
  const download = now.download;
  const percent = download
    ? Math.floor((download.downloaded * 100) / Math.max(download.total, 1))
    : 0;
  const modelName =
    modelList.data?.models.find((m) => m.id === download?.modelId)?.name ??
    download?.modelId;

  return (
    <Stack gap="lg">
      {now.blocked && (
        <Alert color="yellow" role="alert">
          <Stack gap="xs">
            <Text size="sm">
              {problem(t, `writing.blocked.${now.blocked}` as Problem)}
            </Text>
            {/* Each reason with its fix: Settings, where every other one is set. */}
            {now.blocked === "paused" ? (
              <Button
                size="xs"
                variant="light"
                color="yellow"
                w="fit-content"
                onClick={() => void setWritingPaused(false).then(refresh)}
              >
                {t("activity.resume")}
              </Button>
            ) : (
              <Button
                size="xs"
                variant="light"
                color="yellow"
                w="fit-content"
                onClick={openSettings}
              >
                {t("activity.openSettings")}
              </Button>
            )}
          </Stack>
        </Alert>
      )}
      {nothing && <Text size="sm">{t("activity.idle")}</Text>}

      {(running.length > 0 || now.preview || download || building) && (
        <Section title={t("activity.running")}>
          {running.map((job) => (
            <li key={`${job.characterId}|${slot(job.kind, job.startDay)}`}>
              <Group justify="space-between" wrap="nowrap">
                <Stack gap={0}>
                  <Anchor
                    component="button"
                    size="sm"
                    ta="left"
                    onClick={() => openDay(job)}
                  >
                    {label(job)}
                  </Anchor>
                  <Text size="xs" c="dimmed">
                    {words > 0
                      ? t("activity.words", { count: words })
                      : t("activity.writing")}
                  </Text>
                </Stack>
                <Button
                  size="compact-xs"
                  variant="subtle"
                  color="red"
                  onClick={() =>
                    void cancelWriting(
                      job.characterId,
                      job.kind,
                      job.startDay,
                    ).then(refresh)
                  }
                >
                  {t("activity.stop")}
                </Button>
              </Group>
            </li>
          ))}
          {now.preview && (
            <li>
              <Group justify="space-between" wrap="nowrap">
                <Stack gap={0}>
                  <Text size="sm">
                    {t("activity.preview", { narrator: now.preview.narrator })}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {now.preview.running
                      ? t("activity.writing")
                      : t("activity.previewWaiting")}
                  </Text>
                </Stack>
                <Button
                  size="compact-xs"
                  variant="subtle"
                  color="red"
                  onClick={() => void cancelPreview().then(refresh)}
                >
                  {t("activity.stop")}
                </Button>
              </Group>
            </li>
          )}
          {download && (
            <li>
              <Group justify="space-between" wrap="nowrap">
                <Stack gap={4} style={{ flex: 1 }}>
                  <Anchor
                    component="button"
                    size="sm"
                    ta="left"
                    onClick={openSettings}
                  >
                    {t("activity.download", { model: modelName })} · {percent}%
                  </Anchor>
                  <Progress value={percent} size="sm" />
                </Stack>
                <Button
                  size="compact-xs"
                  variant="subtle"
                  color="red"
                  onClick={() => void cancelDownload().then(refresh)}
                >
                  {t("activity.stop")}
                </Button>
              </Group>
            </li>
          )}
          {building && (
            <li>
              <Anchor component="button" size="sm" onClick={openSettings}>
                {t("activity.site")}
              </Anchor>
            </li>
          )}
        </Section>
      )}

      {waiting.length > 0 && (
        <Section title={t("activity.waiting", { count: waiting.length })}>
          {waiting.map((job) => (
            <li key={`${job.characterId}|${slot(job.kind, job.startDay)}`}>
              <Group justify="space-between" wrap="nowrap">
                <Anchor
                  component="button"
                  size="sm"
                  ta="left"
                  onClick={() => openDay(job)}
                >
                  {label(job)}
                </Anchor>
                <Text size="xs" c="dimmed">
                  #{job.position ?? ""}
                </Text>
              </Group>
            </li>
          ))}
        </Section>
      )}

      {failed.length > 0 && (
        <Section title={t("activity.couldNotWrite", { count: failed.length })}>
          {failed.map((job) => (
            <li key={`${job.characterId}|${slot(job.kind, job.startDay)}`}>
              <Group justify="space-between" wrap="nowrap">
                <Stack gap={0}>
                  <Anchor
                    component="button"
                    size="sm"
                    ta="left"
                    onClick={() => openDay(job)}
                  >
                    {label(job)}
                  </Anchor>
                  <Text size="xs" c="dimmed">
                    {problem(
                      t,
                      (job.lastError === "cancelled"
                        ? "writing.reason.cancelled"
                        : job.lastError === "timedOut"
                          ? "writing.reason.timedOut"
                          : "writing.reason.other") as Problem,
                      { message: job.lastError ?? "" },
                    )}
                  </Text>
                </Stack>
                <Button
                  size="compact-xs"
                  variant="light"
                  onClick={() =>
                    void retryWriting(
                      job.characterId,
                      job.kind,
                      job.startDay,
                    ).then(refresh)
                  }
                >
                  {t("activity.retry")}
                </Button>
              </Group>
            </li>
          ))}
        </Section>
      )}

      {/* Paused, the warning above offers Resume; otherwise Pause is here. */}
      {!now.paused && (
        <Button
          variant="default"
          size="xs"
          onClick={() => void setWritingPaused(true).then(refresh)}
        >
          {t("activity.pause")}
        </Button>
      )}
    </Stack>
  );
}
