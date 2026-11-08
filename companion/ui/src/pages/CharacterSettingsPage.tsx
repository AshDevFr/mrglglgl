import { Button, Stack, Text, Title } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CharacterTop } from "@/components/CharacterTop";
import { CharacterWritingForm } from "@/components/CharacterWritingForm";
import { DataUseDialog } from "@/components/DataUse";

/** A character's own settings: how its entries are written, and what its narrator may use. */
export function CharacterSettingsPage({
  characterId,
}: {
  characterId: number;
}) {
  const { t } = useTranslation();
  const [choosingData, setChoosingData] = useState(false);
  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <Stack gap="lg">
        <CharacterTop characterId={characterId} current="settings" />
        <Stack gap="xs">
          <Title order={2} size="h3">
            {t("characterSettings.title")}
          </Title>
          <CharacterWritingForm characterId={characterId} />
        </Stack>
        <Stack gap="xs" align="flex-start">
          <Title order={2} size="h3">
            {t("characterSettings.dataTitle")}
          </Title>
          <Text size="sm" c="dimmed">
            {t("characterSettings.dataHelp")}
          </Text>
          <Button variant="default" onClick={() => setChoosingData(true)}>
            {t("characterSettings.dataButton")}
          </Button>
        </Stack>
      </Stack>
      <DataUseDialog
        characterId={characterId}
        opened={choosingData}
        onClose={() => setChoosingData(false)}
      />
    </div>
  );
}
