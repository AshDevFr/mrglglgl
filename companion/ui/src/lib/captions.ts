import type { TFunction } from "i18next";
import type { Journal } from "@/bindings/Journal";

/** "Level up in Red Rocks, 9:41 PM": what a screenshot caught, where the day's events place
 * it, and when. "In Mulgore, 3:12 PM" for an ordinary screenshot. */
export function photoCaption(
  t: TFunction,
  lang: string,
  journal: Journal,
  day: string,
  photo: { moment: string | null; taken: number },
): string {
  const before = (journal.days[day]?.events ?? []).filter(
    (e) => e.time <= photo.taken && (e.subzone || e.zone),
  );
  const last = before[before.length - 1];
  const place = last ? last.subzone || last.zone : null;
  const time = new Intl.DateTimeFormat(lang, { timeStyle: "short" }).format(
    new Date(photo.taken * 1000),
  );
  if (!photo.moment || photo.moment === "other") {
    return place
      ? t("export.captionPlace", { place, time })
      : t("export.captionTime", { time });
  }
  const said = t(`photo.moment.${photo.moment}`, {
    defaultValue: t("photo.moment.other"),
  });
  const moment = said.charAt(0).toLocaleUpperCase(lang) + said.slice(1);
  return place
    ? t("export.caption", { moment, place, time })
    : t("export.captionNoPlace", { moment, time });
}
