import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { narrator } from "@/api/commands";
import { BackButton } from "@/components/BackButton";
import { EmptyState } from "@/components/EmptyState";
import { NarratorEditor } from "@/components/NarratorEditor";

export function NarratorPage({ narratorId }: { narratorId: string }) {
  const { t } = useTranslation();
  const profile = useQuery({
    queryKey: ["narrator", narratorId],
    queryFn: () => narrator(narratorId),
  });
  return (
    <div className="desk" style={{ padding: "2rem 2.5rem 4rem" }}>
      <BackButton />
      <div style={{ maxWidth: "48rem" }}>
        <h2 className="reading">{t("narrators.editorTitle")}</h2>
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
