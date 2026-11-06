import { Button } from "@mantine/core";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

/** Back to the page the player came from, so the day they were reading stays open; to the
 * character's journal when there is no history, as after a reload. */
export function BackButton({ characterId }: { characterId: number }) {
  const { t } = useTranslation();
  const router = useRouter();
  const navigate = useNavigate();
  return (
    <Button
      variant="subtle"
      color="gray"
      mb="md"
      onClick={() =>
        router.history.canGoBack()
          ? router.history.back()
          : navigate({
              to: "/characters/$characterId",
              params: { characterId: String(characterId) },
            })
      }
    >
      ← {t("nav.back")}
    </Button>
  );
}
