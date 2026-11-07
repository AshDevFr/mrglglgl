import { Switch } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { useWowheadLinks } from "@/lib/wowhead";

/** Whether names in "What happened" link to Wowhead. */
export function WowheadField() {
  const { t } = useTranslation();
  const { on, set } = useWowheadLinks();
  return (
    <Switch
      label={t("settings.wowhead")}
      description={t("settings.wowheadHelp")}
      checked={on}
      onChange={(e) => set(e.currentTarget.checked)}
    />
  );
}
