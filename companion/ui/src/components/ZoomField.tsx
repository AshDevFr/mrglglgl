import { Select } from "@mantine/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { applyZoom, savedZoom, ZOOM_CHANGED, ZOOMS } from "@/lib/zoom";

/** The interface size, for this computer; the keyboard shortcuts change the same setting. */
export function ZoomField() {
  const { t } = useTranslation();
  const [zoom, setZoom] = useState(savedZoom);
  useEffect(() => {
    const follow = (e: Event) => setZoom((e as CustomEvent<number>).detail);
    window.addEventListener(ZOOM_CHANGED, follow);
    return () => window.removeEventListener(ZOOM_CHANGED, follow);
  }, []);
  return (
    <Select
      label={t("settings.zoom")}
      description={t("settings.zoomHelp")}
      data={ZOOMS.map((z) => ({ value: String(z), label: `${z}%` }))}
      value={String(zoom)}
      onChange={(v) => v && void applyZoom(Number(v))}
      allowDeselect={false}
      w={200}
    />
  );
}
