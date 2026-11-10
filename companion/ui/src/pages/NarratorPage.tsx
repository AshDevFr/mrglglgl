import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { blankNarrator, narrator } from "@/api/commands";
import { BackButton } from "@/components/BackButton";
import { EmptyState } from "@/components/EmptyState";
import { NarratorEditor } from "@/components/NarratorEditor";
import { NEW_NARRATOR } from "@/lib/narrators";

export function NarratorPage({ narratorId }: { narratorId: string }) {
  const { t } = useTranslation();
  const profile = useQuery({
    queryKey: ["narrator", narratorId],
    queryFn: () =>
      narratorId === NEW_NARRATOR ? blankNarrator() : narrator(narratorId),
    // A new narrator starts blank each time; saving gives it an id of its own.
    gcTime: narratorId === NEW_NARRATOR ? 0 : undefined,
  });
  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <BackButton />
      <div style={{ maxWidth: "48rem" }}>
        <h1 className="page-title">{t("narrators.editorTitle")}</h1>
        {profile.isPending ? (
          <EmptyState title={t("loading")} />
        ) : profile.data ? (
          <NarratorEditor key={narratorId} initial={profile.data} />
        ) : (
          <EmptyState title={t("narrators.removed")} />
        )}
      </div>
    </div>
  );
}
