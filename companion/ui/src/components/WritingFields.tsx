import { Select, Stack, Switch, Textarea, Title } from "@mantine/core";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { setWritingPaused } from "@/api/commands";
import type { Meddling } from "@/bindings/Meddling";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { WritingSettings } from "@/bindings/WritingSettings";
import { narratorOption, narratorSection } from "@/components/NarratorIcon";

const MEDDLING: Meddling[] = ["off", "rare", "sometimes", "often"];
const SAME = "same";

interface Props {
  value: WritingSettings;
  onChange: (value: WritingSettings) => void;
  narrators: NarratorInfo[];
}

/** How entries are written: narrator, margin notes, language, preferences and chapters. */
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
        renderOption={narratorOption(narrators)}
        leftSection={narratorSection(narrators, value.narrator)}
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
      <Textarea
        label={t("settings.preferences")}
        description={t("settings.preferencesHelp")}
        placeholder={t("settings.preferencesPlaceholder")}
        value={value.preferences}
        onChange={(e) => set({ preferences: e.currentTarget.value })}
        maxLength={500}
        autosize
        minRows={5}
        maxRows={12}
      />
      <Switch
        label={t("settings.weekly")}
        description={t("settings.weeklyHelp")}
        checked={value.weekly}
        onChange={(e) => set({ weekly: e.currentTarget.checked })}
      />
      <Switch
        label={t("settings.review")}
        description={t("settings.reviewHelp")}
        checked={value.review}
        onChange={(e) => set({ review: e.currentTarget.checked })}
      />
      <Switch
        label={t("settings.copyFooter")}
        checked={value.copyFooter}
        onChange={(e) => set({ copyFooter: e.currentTarget.checked })}
      />
    </Stack>
  );
}

/** When the model may run: while the game runs, on battery, or not at all for now. */
export function RunFields({ value, onChange }: Omit<Props, "narrators">) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const set = (patch: Partial<WritingSettings>) =>
    onChange({ ...value, ...patch });
  return (
    <Stack>
      <Title order={2} size="h3">
        {t("settings.runTitle")}
      </Title>
      <Switch
        label={t("settings.writeWhilePlaying")}
        description={t("settings.writeWhilePlayingHelp")}
        checked={value.writeWhilePlaying}
        onChange={(e) => set({ writeWhilePlaying: e.currentTarget.checked })}
      />
      <Switch
        label={t("settings.writeOnBattery")}
        description={t("settings.writeOnBatteryHelp")}
        checked={value.writeOnBattery}
        onChange={(e) => set({ writeOnBattery: e.currentTarget.checked })}
      />
      <Switch
        label={t("settings.paused")}
        description={t("settings.pausedHelp")}
        checked={value.paused}
        onChange={(e) => {
          // Applied at once, as Pause in Activity is, and kept with the other settings.
          const paused = e.currentTarget.checked;
          set({ paused });
          void setWritingPaused(paused).then(() =>
            Promise.all(
              [["activity"], ["writing"]].map((queryKey) =>
                queryClient.invalidateQueries({ queryKey }),
              ),
            ),
          );
        }}
      />
    </Stack>
  );
}
