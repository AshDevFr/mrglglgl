import { Image, Modal } from "@mantine/core";
import { type ComponentProps, useState } from "react";
import { useTranslation } from "react-i18next";
import { screenshotUrl } from "@/api/commands";
import type { Award } from "@/bindings/Award";
import type { Journal } from "@/bindings/Journal";
import type { Screenshot } from "@/bindings/Screenshot";
import { dayFactRows } from "@/lib/factList";
import { formatDay } from "@/lib/format";
import { dayFacts } from "@/lib/journal";
import { splitStamps } from "@/lib/milestones";
import { daySentences } from "@/lib/sentences";
import { EntryPanel } from "./EntryPanel";
import { FactList } from "./FactList";
import { InkStamp } from "./InkStamp";

const TILTS = [-7, 5, -3, 8];

/** A day's screenshots, pinned to the page; one opens full size. */
export function PinnedPhotos({ screenshots }: { screenshots: Screenshot[] }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState<Screenshot | null>(null);
  const label = (s: Screenshot) =>
    t("photo.alt", {
      moment: t(`photo.moment.${s.moment ?? "other"}`, {
        defaultValue: t("photo.moment.other"),
      }),
    });
  return (
    <>
      <div className="photos">
        {screenshots.map((s) => (
          <button
            key={s.path}
            type="button"
            className="photo"
            onClick={() => setOpen(s)}
          >
            <img src={screenshotUrl(s.path)} alt={label(s)} />
          </button>
        ))}
      </div>
      <Modal
        opened={open !== null}
        onClose={() => setOpen(null)}
        size="auto"
        title={open ? label(open) : undefined}
      >
        {open && <Image src={screenshotUrl(open.path)} alt={label(open)} />}
      </Modal>
    </>
  );
}

/** The day's entry and everything needed to write it, when writing is set up. */
export type DayWriting = Omit<
  ComponentProps<typeof EntryPanel>,
  "kind" | "day" | "facts"
>;

interface Props {
  journal: Journal;
  day: string;
  awards: Award[];
  screenshots?: Screenshot[];
  writing?: DayWriting;
}

/** One day as a journal page: the date, the entry (or what happened in short sentences while
 * there is none), stamps and photos. */
export function JournalPage({
  journal,
  day,
  awards,
  screenshots = [],
  writing,
}: Props) {
  const { t, i18n } = useTranslation();
  const facts = dayFacts(journal, journal.days[day]);
  const sentences = daySentences(facts, t, i18n.language);
  // A character's first days can earn many stamps: the rarest are pressed, the rest listed.
  const { pressed: stamps, more } = splitStamps(
    awards.filter((a) => a.day === day),
  );
  // The day as recorded, item by item; when nothing was, the plain sentence that says so.
  const plain = (
    <FactList
      rows={facts.nothingHappened ? [] : dayFactRows(facts, t)}
      empty={sentences.join(" ")}
    />
  );
  return (
    <article
      className="journal-page"
      aria-label={formatDay(day, i18n.language, true)}
    >
      {stamps.length > 0 && (
        <div className="stamps">
          {stamps.map((a, i) => (
            <InkStamp key={a.id} award={a} tilt={TILTS[i % TILTS.length]} />
          ))}
        </div>
      )}
      {/* On a wide window the photos and milestones sit beside the entry, not under it. */}
      <div className="page-columns">
        <div className="page-main">
          <h2>{formatDay(day, i18n.language, true)}</h2>
          {writing ? (
            <EntryPanel kind="day" day={day} facts={plain} {...writing} />
          ) : (
            plain
          )}
        </div>
        {(screenshots.length > 0 || stamps.length > 0) && (
          <aside className="page-side">
            {screenshots.length > 0 && (
              <PinnedPhotos screenshots={screenshots} />
            )}
            {stamps.length > 0 && (
              <section className="more-milestones">
                <h3>{t("milestones.ofTheDay")}</h3>
                <ul>
                  {/* Every milestone of the day, the stamped ones first: the stamps are small. */}
                  {[...stamps, ...more].map((a) => (
                    <li key={a.id}>
                      <strong>{t(`milestones.${a.id}.name`)}</strong>
                      {": "}
                      {t(`milestones.${a.id}.description`)}
                    </li>
                  ))}
                </ul>
              </section>
            )}
          </aside>
        )}
      </div>
      {facts.compacted && <p className="note">{t("character.compacted")}</p>}
    </article>
  );
}
