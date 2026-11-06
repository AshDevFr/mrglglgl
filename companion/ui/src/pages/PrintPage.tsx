import { Alert, Button, Group } from "@mantine/core";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { printPage } from "@/api/commands";
import { EmptyState } from "@/components/EmptyState";
import { ExportDocument } from "@/components/ExportDocument";
import { problem } from "@/i18n/problems";
import { loadExportData } from "@/lib/exportPage";
import { savedPaper } from "@/lib/paper";
import "@/styles/export.css";

/** The journal laid out for paper, on its own, outside the app's frame. The print dialog opens
 * once the pictures are in, where the player saves it as a PDF. */
export function PrintPage({ characterId }: { characterId: number }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const data = useQuery({
    queryKey: ["export", characterId],
    queryFn: () => loadExportData(characterId, false),
  });
  const page = useRef<HTMLDivElement>(null);
  const asked = useRef(false);

  useEffect(() => {
    if (!data.data || asked.current || !page.current) return;
    asked.current = true;
    const pictures = [...page.current.querySelectorAll("img")];
    void Promise.all(
      pictures.map((img) =>
        img.complete
          ? null
          : new Promise((done) => {
              img.onload = done;
              img.onerror = done;
            }),
      ),
    ).then(() => printPage());
  }, [data.data]);

  const back = () =>
    navigate({
      to: "/characters/$characterId",
      params: { characterId: String(characterId) },
    });

  return (
    <div
      ref={page}
      className="paper-light print-sheet"
      data-paper={data.data?.paper ?? savedPaper()}
    >
      <Group className="no-print" justify="space-between" p="md">
        <Button variant="subtle" color="gray" onClick={back}>
          ← {t("nav.back")}
        </Button>
        <Button onClick={() => void printPage()} disabled={!data.data}>
          {t("share.print")}
        </Button>
      </Group>
      {data.isError ? (
        <Alert color="red" m="md">
          {problem(t, "share.exportFailed", { message: String(data.error) })}
        </Alert>
      ) : data.data ? (
        <ExportDocument {...data.data} />
      ) : (
        <EmptyState title={t("loading")} />
      )}
    </div>
  );
}
