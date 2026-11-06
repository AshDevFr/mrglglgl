import {
  Alert,
  Button,
  Group,
  PasswordInput,
  Radio,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  endpointSendsOffMachine,
  hasApiKey,
  saveApiKey,
  testEndpoint,
} from "@/api/commands";
import type { ProviderChoice } from "@/bindings/ProviderChoice";
import type { WritingSettings } from "@/bindings/WritingSettings";

interface Props {
  value: WritingSettings;
  onChange: (value: WritingSettings) => void;
}

function host(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** Who writes: this computer, or an endpoint, with a plain statement of what an endpoint
 * elsewhere receives. */
export function EndpointFields({ value, onChange }: Props) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [key, setKey] = useState("");
  const [test, setTest] = useState<
    { state: "idle" | "testing" | "ok" } | { state: "failed"; message: string }
  >({ state: "idle" });
  const set = (patch: Partial<WritingSettings>) =>
    onChange({ ...value, ...patch });
  const url = value.endpointUrl ?? "";
  const offMachine = useQuery({
    queryKey: ["offMachine", url],
    queryFn: () => endpointSendsOffMachine(url),
    enabled: url.trim() !== "",
  });
  const keySaved = useQuery({ queryKey: ["apiKey"], queryFn: hasApiKey });

  async function storeKey(k: string | null) {
    await saveApiKey(k);
    setKey("");
    await queryClient.invalidateQueries({ queryKey: ["apiKey"] });
  }

  async function runTest() {
    setTest({ state: "testing" });
    try {
      await testEndpoint(url, value.endpointModel ?? "");
      setTest({ state: "ok" });
    } catch (e) {
      setTest({ state: "failed", message: String(e) });
    }
  }

  return (
    <Stack>
      <Title order={2} size="h3">
        {t("settings.providerTitle")}
      </Title>
      <Radio.Group
        value={value.provider}
        onChange={(v) => set({ provider: v as ProviderChoice })}
      >
        <Stack gap="xs">
          <Radio value="local" label={t("settings.provider.local")} />
          <Radio value="endpoint" label={t("settings.provider.endpoint")} />
        </Stack>
      </Radio.Group>
      {value.provider === "endpoint" && (
        <Stack>
          <TextInput
            label={t("settings.endpointUrl")}
            description={t("settings.endpointUrlHelp")}
            value={url}
            onChange={(e) => set({ endpointUrl: e.currentTarget.value })}
          />
          {offMachine.data && (
            <Alert color="yellow" role="note">
              {t("settings.disclosure", { host: host(url) })}
            </Alert>
          )}
          <TextInput
            label={t("settings.endpointModel")}
            value={value.endpointModel ?? ""}
            onChange={(e) => set({ endpointModel: e.currentTarget.value })}
          />
          <PasswordInput
            label={t("settings.apiKey")}
            description={
              keySaved.data
                ? `${t("settings.apiKeyHelp")} ${t("settings.apiKeySaved")}`
                : t("settings.apiKeyHelp")
            }
            value={key}
            onChange={(e) => setKey(e.currentTarget.value)}
          />
          <Group gap="xs">
            <Button
              variant="light"
              disabled={key.trim() === ""}
              onClick={() => void storeKey(key.trim())}
            >
              {t("settings.saveKey")}
            </Button>
            {keySaved.data && (
              <Button
                variant="subtle"
                color="gray"
                onClick={() => void storeKey(null)}
              >
                {t("settings.removeKey")}
              </Button>
            )}
            <Button
              variant="default"
              disabled={url.trim() === "" || !value.endpointModel}
              loading={test.state === "testing"}
              onClick={() => void runTest()}
            >
              {t("settings.testConnection")}
            </Button>
          </Group>
          {test.state === "ok" && (
            <Text size="sm" c="green" role="status">
              {t("settings.connectionOk")}
            </Text>
          )}
          {test.state === "failed" && (
            <Text size="sm" c="red" role="status">
              {t("settings.connectionFailed", { message: test.message })}
            </Text>
          )}
          <Switch
            label={t("settings.pseudonyms")}
            description={t("settings.pseudonymsHelp")}
            checked={value.pseudonyms}
            onChange={(e) => set({ pseudonyms: e.currentTarget.checked })}
          />
        </Stack>
      )}
    </Stack>
  );
}
