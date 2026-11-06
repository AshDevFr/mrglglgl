import { Select, Stack, Switch, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { Meddling } from "@/bindings/Meddling";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { WritingSettings } from "@/bindings/WritingSettings";

const MEDDLING: Meddling[] = ["off", "rare", "sometimes", "often"];
const SAME = "same";

interface Props {
  value: WritingSettings;
  onChange: (value: WritingSettings) => void;
  narrators: NarratorInfo[];
}

/** How entries are written: narrator, margin notes, language, and when. */
export function WritingFields({ value, onChange, narrators }: Props) {
  const { t } = useTranslation();
  const set = (patch: Partial<WritingSettings>) =>
    onChange({ ...value, ...patch });
  return (
    <Stack>
      <Title order={2} size="h3">
        {t("settings.writingTitle")}
      </Title>
      <Select
        label={t("settings.narrator")}
        description={t("settings.narratorHelp")}
        data={narrators.map((n) => ({
          value: n.id,
          label: n.optIn ? t("settings.optIn", { name: n.name }) : n.name,
        }))}
        value={value.narrator}
        onChange={(v) => v && set({ narrator: v })}
        allowDeselect={false}
      />
      <Select
        label={t("settings.meddling")}
        description={t("settings.meddlingHelp")}
        data={MEDDLING.map((m) => ({
          value: m,
          label: t(`settings.meddlingLevel.${m}`),
        }))}
        value={value.meddling}
        onChange={(v) => v && set({ meddling: v as Meddling })}
        allowDeselect={false}
      />
      <Select
        label={t("settings.proseLanguage")}
        data={[
          { value: SAME, label: t("settings.proseLanguageSame") },
          { value: "en", label: t("settings.languageEnglish") },
          { value: "fr", label: t("settings.languageFrench") },
        ]}
        value={value.proseLanguage ?? SAME}
        onChange={(v) => set({ proseLanguage: !v || v === SAME ? null : v })}
        allowDeselect={false}
      />
      <Switch
        label={t("settings.writeWhilePlaying")}
        description={t("settings.writeWhilePlayingHelp")}
        checked={value.writeWhilePlaying}
        onChange={(e) => set({ writeWhilePlaying: e.currentTarget.checked })}
      />
      <Switch
        label={t("settings.paused")}
        checked={value.paused}
        onChange={(e) => set({ paused: e.currentTarget.checked })}
      />
      <Switch
        label={t("settings.copyFooter")}
        checked={value.copyFooter}
        onChange={(e) => set({ copyFooter: e.currentTarget.checked })}
      />
    </Stack>
  );
}
