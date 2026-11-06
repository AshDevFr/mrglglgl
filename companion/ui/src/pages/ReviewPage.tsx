import { useTranslation } from "react-i18next";
import { CharacterTitle } from "@/components/CharacterTitle";
import { ReviewList } from "@/components/ReviewList";

export function ReviewPage({ characterId }: { characterId: number }) {
  const { t } = useTranslation();
  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <CharacterTitle characterId={characterId} current="review" />
      <div style={{ maxWidth: "48rem" }}>
        <h2 className="reading">{t("review.title")}</h2>
        <ReviewList characterId={characterId} />
      </div>
    </div>
  );
}
