import {
  Alert,
  Button,
  Checkbox,
  Code,
  Group,
  ScrollArea,
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

/** A warning or an error, as the log writes its level. */
export function isProblem(line: string): boolean {
  return / (WARN|ERROR) /.test(line);
}

/** The app's log, for a player asking for help: its last lines, warnings and errors on their own
 * if asked, a button to copy them, and the file itself. */
export function LogPanel() {
  const { t } = useTranslation();
  const [onlyProblems, setOnlyProblems] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const log = useQuery({
    queryKey: ["log"],
    queryFn: () => recentLog(LINES),
  });
  const lines = (log.data ?? []).filter((l) => !onlyProblems || isProblem(l));
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
      <Checkbox
        label={t("log.onlyProblems")}
        checked={onlyProblems}
        onChange={(e) => setOnlyProblems(e.currentTarget.checked)}
      />
      {error && <Alert color="red">{error}</Alert>}
      <ScrollArea h={420} type="auto" offsetScrollbars>
        {lines.length === 0 ? (
          <Text size="sm" c="dimmed">
            {onlyProblems ? t("log.noProblems") : t("log.empty")}
          </Text>
        ) : (
          <Code block aria-label={t("log.title")}>
            {lines.join("\n")}
          </Code>
        )}
      </ScrollArea>
    </Stack>
  );
}
