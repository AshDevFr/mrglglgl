import { useTranslation } from "react-i18next";
import type { Award } from "@/bindings/Award";
import type { Journal } from "@/bindings/Journal";
import type { Story } from "@/bindings/Story";
import { formatDay } from "@/lib/format";
import { dayFacts } from "@/lib/journal";
import type { Paper } from "@/lib/paper";
import { daySentences } from "@/lib/sentences";
import { weekStart } from "@/lib/weeks";
import { versionsOf } from "@/lib/writing";
import { EntryText } from "./EntryPanel";
import { InkStamp } from "./InkStamp";

const TILTS = [-7, 5, -3, 8];

/** A screenshot in an export: where its picture comes from, and what it caught. */
export interface ExportPhoto {
  src: string;
  /** What the addon was capturing, e.g. `"levelUp"`; `null` for an ordinary screenshot. */
  moment: string | null;
  /** Unix time. */
  taken: number;
}

export interface ExportData {
  title: string;
  journal: Journal;
  /** Accepted stories; the newest of each day and week that passed the fact check is told. */
  stories: Story[];
  awards: Award[];
  /** Picture sources by day: embedded data for a file, asset URLs for printing. */
  photos: Record<string, ExportPhoto[]>;
  /** The paper chosen in Settings: what an exported page starts on, and what prints. */
  paper: Paper;
}

/** The whole journal as a book, oldest day first, with the pages, stamps and photos the app
 * shows. Plain markup and the paper styles only, so it stands alone as a web page or on paper;
 * the paper and theme come from the element around it. */
export function ExportDocument({
  title,
  journal,
  stories,
  awards,
  photos,
}: ExportData) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const told = (kind: Story["kind"], day: string) =>
    versionsOf(stories, kind, day).find((s) => s.check !== "failed");
  const days = Object.keys(journal.days).sort();
  const weeks: [string, string[]][] = [];
  for (const day of days) {
    const monday = weekStart(day);
    const last = weeks[weeks.length - 1];
    if (last && last[0] === monday) last[1].push(day);
    else weeks.push([monday, [day]]);
  }
  /** "Level up in Red Rocks, 9:41 PM": what was happening, where the day's events place it. */
  const caption = (day: string, photo: ExportPhoto) => {
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
  };
  const c = journal.character;
  const meta = [c.race, c.realm].filter(Boolean).join(" · ");

  return (
    <main className="export">
      <header className="export-header">
        <h1>{title}</h1>
        {meta && <p>{meta}</p>}
      </header>
      {weeks.map(([monday, weekDays]) => {
        const chapter = told("week", monday);
        return (
          <section key={monday} className="export-week">
            <p className="export-week-label">
              {t("export.weekOf", { date: formatDay(monday, lang, true) })}
            </p>
            {chapter && (
              <div className="export-chapter">
                <EntryText text={chapter.text} />
              </div>
            )}
            {weekDays.map((day) => {
              const entry = told("day", day);
              const stamps = awards.filter((a) => a.day === day);
              const pictures = photos[day] ?? [];
              const heading = formatDay(day, lang, true);
              return (
                <article
                  key={day}
                  className="journal-page"
                  aria-label={heading}
                >
                  {stamps.length > 0 && (
                    <div className="stamps">
                      {stamps.map((a, i) => (
                        <InkStamp
                          key={a.id}
                          award={a}
                          tilt={TILTS[i % TILTS.length]}
                        />
                      ))}
                    </div>
                  )}
                  <h2>{heading}</h2>
                  {entry ? (
                    <EntryText text={entry.text} />
                  ) : (
                    <p className="entry">
                      {daySentences(
                        dayFacts(journal, journal.days[day]),
                        t,
                        lang,
                      ).join(" ")}
                    </p>
                  )}
                  {pictures.length > 0 && (
                    <div className="photos">
                      {pictures.map((photo) => {
                        const words = caption(day, photo);
                        return (
                          <figure
                            key={photo.taken + photo.src.slice(-24)}
                            className="photo"
                          >
                            <img src={photo.src} alt={words} />
                            <figcaption>{words}</figcaption>
                          </figure>
                        );
                      })}
                    </div>
                  )}
                </article>
              );
            })}
          </section>
        );
      })}
      <footer className="export-footer">{t("export.writtenWith")}</footer>
    </main>
  );
}
