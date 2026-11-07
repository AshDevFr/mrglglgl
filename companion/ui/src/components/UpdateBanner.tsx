import { Alert, Button, Group, Text } from "@mantine/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { problem } from "@/i18n/problems";
import { availableUpdate, installAndRestart } from "@/lib/updates";

/** Offers a newer version of the app, once per launch, without nagging: "Later" hides it until the
 * next start. */
export function UpdateBanner() {
  const { t } = useTranslation();
  const [later, setLater] = useState(false);
  const update = useQuery({
    queryKey: ["update"],
    queryFn: availableUpdate,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  const install = useMutation({
    mutationFn: () =>
      installAndRestart(update.data as NonNullable<typeof update.data>),
  });
  if (later || !update.data) return null;
  return (
    <Alert
      color="teal"
      variant="light"
      m="md"
      title={t("update.title", { version: update.data.version })}
    >
      <Text size="sm">{t("update.body")}</Text>
      {install.isError && (
        <Text size="sm" c="red">
          {problem(t, "update.failed")}
        </Text>
      )}
      <Group mt="sm" gap="xs">
        <Button
          size="xs"
          loading={install.isPending}
          onClick={() => install.mutate()}
        >
          {t("update.install")}
        </Button>
        <Button
          size="xs"
          variant="default"
          disabled={install.isPending}
          onClick={() => setLater(true)}
        >
          {t("update.later")}
        </Button>
      </Group>
    </Alert>
  );
}
