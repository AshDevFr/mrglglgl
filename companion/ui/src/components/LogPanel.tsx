import {
  Alert,
  Button,
  Group,
  ScrollArea,
  Select,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { recentLog, showLog } from "@/api/commands";
import { problem } from "@/i18n/problems";

/** How many of the log's last lines the tab reads. */
const LINES = 500;

/** The log's levels, most severe first, as the log writes them in lower case. */
const LEVELS = ["error", "warn", "info", "debug", "trace"] as const;
type Level = (typeof LEVELS)[number];

/** Each line's level. A line without one (the rest of a long message, a cause under an error)
 * belongs to the line above it, so it shows and hides with that line. */
export function levelsOf(lines: string[]): Level[] {
  let current: Level = "info";
  return lines.map((line) => {
    const found = / (ERROR|WARN|INFO|DEBUG|TRACE) /.exec(line)?.[1];
    if (found) current = found.toLowerCase() as Level;
    return current;
  });
}

/** The app's log, for a player asking for help: its last lines, warnings and errors on their own
 * if asked, a button to copy them, and the file itself. */
export function LogPanel() {
  const { t } = useTranslation();
  // Info and above: what the app did, without the detail kept for tracking a problem down.
  const [level, setLevel] = useState<Level>("info");
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const log = useQuery({
    queryKey: ["log"],
    queryFn: () => recentLog(LINES),
  });
  const all = log.data ?? [];
  const levels = levelsOf(all);
  const shown = all
    .map((text, i) => ({ text, level: levels[i] }))
    .filter((l) => LEVELS.indexOf(l.level) <= LEVELS.indexOf(level));
  const lines = shown.map((l) => l.text);
  async function show() {
    setError(null);
    try {
      await showLog();
    } catch (e) {
      setError(problem(t, "log.showFailed", { message: String(e) }));
    }
  }
  async function copy() {
    setError(null);
    try {
      await navigator.clipboard.writeText(lines.join("\n"));
      setCopied(true);
    } catch (e) {
      setError(problem(t, "log.copyFailed", { message: String(e) }));
    }
  }
  return (
    <Stack gap="sm">
      <Title order={2} size="h3">
        {t("log.title")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("log.help")}
      </Text>
      <Group gap="xs">
        <Button variant="default" onClick={() => void log.refetch()}>
          {t("log.refresh")}
        </Button>
        <Button variant="default" onClick={() => void copy()}>
          {copied ? t("log.copied") : t("log.copy")}
        </Button>
        <Button variant="default" onClick={() => void show()}>
          {t("log.show")}
        </Button>
      </Group>
      <Select
        label={t("log.level")}
        description={t("log.levelHelp")}
        data={LEVELS.map((value) => ({
          value,
          // Keyed as the log spells them.
          label: t(`log.levels.${value.toUpperCase()}`),
        }))}
        value={level}
        onChange={(v) => v && setLevel(v as Level)}
        allowDeselect={false}
        maw={260}
      />
      {error && <Alert color="red">{error}</Alert>}
      <ScrollArea h={420} type="auto" offsetScrollbars>
        {shown.length === 0 ? (
          <Text size="sm" c="dimmed">
            {all.length > 0 ? t("log.nothingAtLevel") : t("log.empty")}
          </Text>
        ) : (
          <pre className="log-lines" role="log" aria-label={t("log.title")}>
            {shown.map((l, i) => (
              // Lines repeat, so their place is part of the key.
              // biome-ignore lint/suspicious/noArrayIndexKey: the log is read whole, never reordered
              <span key={i} className="log-line" data-level={l.level}>
                {l.text}
                {"\n"}
              </span>
            ))}
          </pre>
        )}
      </ScrollArea>
    </Stack>
  );
}
