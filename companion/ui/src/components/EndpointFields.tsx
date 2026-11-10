import {
  Alert,
  Autocomplete,
  Button,
  Group,
  PasswordInput,
  Radio,
  Select,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  endpointSendsOffMachine,
  hasApiKey,
  saveApiKey,
  testEndpoint,
} from "@/api/commands";
import type { ProviderChoice } from "@/bindings/ProviderChoice";
import type { WritingSettings } from "@/bindings/WritingSettings";
import { problem } from "@/i18n/problems";
import { ENDPOINT_PRESETS, OTHER, presetFor } from "@/lib/endpoints";

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
    | { state: "idle" | "testing" }
    | { state: "ok"; model: string }
    | { state: "failed"; message: string }
  >({ state: "idle" });
  // What the endpoint lists, offered as the model to pick; a name can still be typed.
  const [models, setModels] = useState<string[] | null>(null);
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
    const model = (value.endpointModel ?? "").trim();
    setTest({ state: "testing" });
    try {
      setModels(await testEndpoint(url, model, key.trim() || undefined));
      setTest({ state: "ok", model });
    } catch (e) {
      setTest({ state: "failed", message: String(e) });
    }
  }

  // An endpoint already set up lists its models on its own, so the picker is ready.
  // biome-ignore lint/correctness/useExhaustiveDependencies: once, for the address it opened with
  useEffect(() => {
    if (value.provider !== "endpoint" || url.trim() === "") return;
    void testEndpoint(url, "")
      .then(setModels)
      .catch(() => {});
  }, [value.provider]);

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
          {/* A known service fills the address in; Other keeps whatever is typed below. */}
          <Select
            label={t("settings.service")}
            data={[
              ...ENDPOINT_PRESETS.map((p) => ({ value: p.id, label: p.name })),
              { value: OTHER, label: t("settings.serviceOther") },
            ]}
            value={presetFor(url)}
            onChange={(id) => {
              const preset = ENDPOINT_PRESETS.find((p) => p.id === id);
              if (preset) set({ endpointUrl: preset.url });
            }}
            allowDeselect={false}
          />
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
              disabled={url.trim() === ""}
              loading={test.state === "testing"}
              onClick={() => void runTest()}
            >
              {t("settings.testConnection")}
            </Button>
          </Group>
          {test.state === "ok" && (
            <Text size="sm" c="green" role="status">
              {test.model
                ? t("settings.connectionOkModel", { model: test.model })
                : models && models.length > 0
                  ? t("settings.connectionOkListed", { count: models.length })
                  : t("settings.connectionOkUnlisted")}
            </Text>
          )}
          {test.state === "failed" && (
            <Text size="sm" c="red" role="status">
              {problem(t, "settings.connectionFailed", {
                message: test.message,
              })}
            </Text>
          )}
          <Autocomplete
            label={t("settings.endpointModel")}
            description={
              models && models.length > 0
                ? t("settings.endpointModelPick")
                : t("settings.endpointModelHelp")
            }
            data={models ?? []}
            value={value.endpointModel ?? ""}
            onChange={(model) => {
              set({ endpointModel: model });
              // A result said of another model no longer holds.
              if (test.state !== "testing") setTest({ state: "idle" });
            }}
            limit={50}
            maxDropdownHeight={280}
          />
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
