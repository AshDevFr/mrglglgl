import { Select } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { applyPaper, PAPERS, type Paper, savedPaper } from "@/lib/paper";

/** The paper the journal is laid on, for this computer; exports carry it too. */
export function PaperField() {
  const { t } = useTranslation();
  const [paper, setPaper] = useState<Paper>(savedPaper);
  return (
    <Select
      label={t("settings.paper")}
      description={t("settings.paperHelp")}
      data={PAPERS.map((p) => ({ value: p, label: t(`settings.papers.${p}`) }))}
      value={paper}
      onChange={(v) => {
        const next = PAPERS.find((p) => p === v);
        if (!next) return;
        applyPaper(next);
        setPaper(next);
      }}
      allowDeselect={false}
      w={260}
    />
  );
}
