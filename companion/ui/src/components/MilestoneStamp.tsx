import { useTranslation } from "react-i18next";
import type { Award } from "@/bindings/Award";
import { formatDay } from "@/lib/format";

interface Props {
  award: Award;
  showDay?: boolean;
}

export function MilestoneStamp({ award, showDay = false }: Props) {
  const { t, i18n } = useTranslation();
  return (
    <span className="stamp" title={t(`milestones.${award.id}.description`)}>
      {t(`milestones.${award.id}.name`)}
      {showDay && (
        <small>
          {t("milestones.earned", { day: formatDay(award.day, i18n.language) })}
        </small>
      )}
    </span>
  );
}
