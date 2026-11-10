import { Loader, NavLink } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { appInfo } from "@/api/commands";
import { problem } from "@/i18n/problems";
import { checkForUpdate } from "@/lib/updates";

/** Sent by the macOS app menu's Check for Updates item. */
export const CHECK_FOR_UPDATES = "check-for-updates";

/** Checks for a newer version from the sidebar, or from the macOS app menu: one found shows in
 * the banner (even after Later), otherwise a notification says it is up to date or why the check
 * failed. */
export function SidebarUpdateCheck() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [checking, setChecking] = useState(false);
  const busy = useRef(false);

  async function look() {
    if (busy.current) return;
    busy.current = true;
    setChecking(true);
    try {
      const { update, failed } = await checkForUpdate();
      if (update) {
        // The banner reads this, and shows it again even after Later.
        queryClient.setQueryData(["update"], update);
      } else if (failed) {
        notifications.show({
          color: "red",
          message: problem(t, "update.checkFailed"),
        });
      } else {
        notifications.show({
          message: t("update.latest", { version: (await appInfo()).version }),
        });
      }
    } finally {
      busy.current = false;
      setChecking(false);
    }
  }

  // The latest `look` for the menu listener, which is registered once.
  const lookRef = useRef(look);
  lookRef.current = look;
  useEffect(() => {
    const stop = listen(CHECK_FOR_UPDATES, () => void lookRef.current());
    return () => {
      void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  return (
    <NavLink
      component="button"
      label={t("update.check")}
      leftSection={checking ? <Loader size={12} /> : undefined}
      disabled={checking}
      onClick={() => void look()}
    />
  );
}
