import { Group, Image, Modal, UnstyledButton } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { screenshotUrl } from "@/api/commands";
import type { Screenshot } from "@/bindings/Screenshot";

/** A day's screenshots as thumbnails; one opens full size on click. */
export function ScreenshotStrip({
  screenshots,
}: {
  screenshots: Screenshot[];
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState<Screenshot | null>(null);
  const label = (s: Screenshot) =>
    t("day.screenshotAlt", {
      moment: t(`day.screenshotMoment.${s.moment ?? "other"}`, {
        defaultValue: t("day.screenshotMoment.other"),
      }),
    });

  return (
    <>
      <Group gap="sm">
        {screenshots.map((s) => (
          <UnstyledButton key={s.path} onClick={() => setOpen(s)}>
            <Image
              src={screenshotUrl(s.path)}
              alt={label(s)}
              w={180}
              radius="sm"
            />
          </UnstyledButton>
        ))}
      </Group>
      <Modal
        opened={open !== null}
        onClose={() => setOpen(null)}
        size="auto"
        title={open ? label(open) : undefined}
      >
        {open && <Image src={screenshotUrl(open.path)} alt={label(open)} />}
      </Modal>
    </>
  );
}
