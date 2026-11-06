import { useTranslation } from "react-i18next";
import { formatDay } from "@/lib/format";
import { EntryPanel } from "./EntryPanel";
import type { DayWriting } from "./JournalPage";

interface Props extends DayWriting {
  /** The week's Monday. */
  monday: string;
}

/** The week told as one chapter, written from the week's entries once the week is over. */
export function WeekChapter({ monday, ...writing }: Props) {
  const { t, i18n } = useTranslation();
  return (
    <section
      className="week-chapter"
      aria-label={t("writing.chapter", {
        date: formatDay(monday, i18n.language, true),
      })}
    >
      <h3>{t("writing.chapterTitle")}</h3>
      <EntryPanel kind="week" day={monday} {...writing} />
    </section>
  );
}
