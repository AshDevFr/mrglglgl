import { Button, Group, TextInput } from "@mantine/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";

interface Props {
  label: string;
  description?: string;
  value: string;
  onChange: (folder: string) => void;
}

/** A folder: chosen with the system's folder picker, or typed. */
export function FolderField({ label, description, value, onChange }: Props) {
  const { t } = useTranslation();
  async function choose() {
    const picked = await open({
      directory: true,
      multiple: false,
      defaultPath: value || undefined,
      title: label,
    });
    if (typeof picked === "string") onChange(picked);
  }
  return (
    <Group gap="xs" align="flex-end" wrap="nowrap">
      <TextInput
        label={label}
        description={description}
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
        style={{ flex: 1 }}
      />
      <Button variant="default" onClick={() => void choose()}>
        {t("share.choose")}
      </Button>
    </Group>
  );
}
