import { Alert, Button, Stack, Text, Title } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { showLog } from "@/api/commands";
import { problem } from "@/i18n/problems";

/** Where the app's log is, for a player asking for help. */
export function LogPanel() {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  async function show() {
    setError(null);
    try {
      await showLog();
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <Stack gap="xs">
      <Title order={2} size="h3">
        {t("log.title")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("log.help")}
      </Text>
      <Button variant="default" w="fit-content" onClick={() => void show()}>
        {t("log.show")}
      </Button>
      {error && (
        <Alert color="red">
          {problem(t, "log.showFailed", { message: error })}
        </Alert>
      )}
    </Stack>
  );
}
