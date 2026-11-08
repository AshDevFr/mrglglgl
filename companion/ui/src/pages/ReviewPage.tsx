import { Stack } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { CharacterTop } from "@/components/CharacterTop";
import { ReviewList } from "@/components/ReviewList";

/** One character's To review list, or, without `characterId`, every character's. */
export function ReviewPage({ characterId }: { characterId?: number }) {
  const { t } = useTranslation();
  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      {characterId !== undefined && (
        <Stack gap="lg" maw="48rem" mb="xl">
          <CharacterTop characterId={characterId} current="review" />
        </Stack>
      )}
      <div style={{ maxWidth: "48rem" }}>
        <h2 className="reading">{t("review.title")}</h2>
        <ReviewList characterId={characterId} />
      </div>
    </div>
  );
}
