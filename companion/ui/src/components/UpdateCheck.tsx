import { Button, Group, Text } from "@mantine/core";
import { useQueryClient } from "@tanstack/react-query";
import type { Update } from "@tauri-apps/plugin-updater";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { appInfo } from "@/api/commands";
import { problem } from "@/i18n/problems";
import { checkForUpdate } from "@/lib/updates";

type Outcome =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest"; version: string }
  | { kind: "failed" };

/** Looks for a newer version when the player asks, and hands one it finds to the banner, which
 * offers to install it even after "Later". */
export function UpdateCheck({
  onFound,
}: {
  onFound?: (update: Update) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [outcome, setOutcome] = useState<Outcome>({ kind: "idle" });

  async function look() {
    setOutcome({ kind: "checking" });
    const { update, failed } = await checkForUpdate();
    if (update) {
      setOutcome({ kind: "idle" });
      // The banner reads this, and shows it again even after Later.
      queryClient.setQueryData(["update"], update);
      onFound?.(update);
    } else if (failed) {
      setOutcome({ kind: "failed" });
    } else {
      setOutcome({ kind: "latest", version: (await appInfo()).version });
    }
  }

  const description =
    outcome.kind === "checking"
      ? t("update.checking")
      : outcome.kind === "latest"
        ? t("update.latest", { version: outcome.version })
        : outcome.kind === "failed"
          ? problem(t, "update.checkFailed")
          : undefined;
  return (
    <Group gap="sm" align="center">
      <Button
        variant="default"
        size="xs"
        loading={outcome.kind === "checking"}
        onClick={() => void look()}
      >
        {t("update.check")}
      </Button>
      {description && (
        <Text
          size="sm"
          c={outcome.kind === "failed" ? "red" : "dimmed"}
          role="status"
        >
          {description}
        </Text>
      )}
    </Group>
  );
}
