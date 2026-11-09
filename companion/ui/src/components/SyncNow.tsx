import { Loader, NavLink } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { syncNow } from "@/api/commands";

/** Reads every character's journal now, and says what it found. */
export function SyncNow() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [reading, setReading] = useState(false);

  async function read() {
    setReading(true);
    try {
      const report = await syncNow();
      await queryClient.invalidateQueries();
      if (report.roots.length === 0) {
        notifications.show({ color: "yellow", message: t("sync.noGame") });
        return;
      }
      const read = t("sync.read", { count: report.imported });
      notifications.show(
        report.unreadable > 0
          ? {
              color: "yellow",
              message: `${read} ${t("sync.unreadableCount", { count: report.unreadable })}`,
            }
          : { message: read },
      );
    } catch (e) {
      notifications.show({ color: "red", message: String(e) });
    } finally {
      setReading(false);
    }
  }

  return (
    <NavLink
      component="button"
      label={t("nav.syncNow")}
      leftSection={reading ? <Loader size={12} /> : undefined}
      disabled={reading}
      onClick={() => void read()}
    />
  );
}
