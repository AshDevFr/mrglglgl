import { Image, Modal } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { screenshotUrl } from "@/api/commands";
import type { Award } from "@/bindings/Award";
import type { Journal } from "@/bindings/Journal";
import type { Screenshot } from "@/bindings/Screenshot";
import { formatDay } from "@/lib/format";
import { dayFacts } from "@/lib/journal";
import { daySentences } from "@/lib/sentences";
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

interface Props {
  journal: Journal;
  day: string;
  awards: Award[];
  screenshots?: Screenshot[];
}

/** One day as a journal page: the date, what happened in short sentences, stamps and photos. */
export function JournalPage({ journal, day, awards, screenshots = [] }: Props) {
  const { t, i18n } = useTranslation();
  const facts = dayFacts(journal, journal.days[day]);
  const sentences = daySentences(facts, t, i18n.language);
  const stamps = awards.filter((a) => a.day === day);
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
      <h2>{formatDay(day, i18n.language, true)}</h2>
      <p className="entry">{sentences.join(" ")}</p>
      {screenshots.length > 0 && <PinnedPhotos screenshots={screenshots} />}
      {facts.compacted && <p className="note">{t("character.compacted")}</p>}
    </article>
  );
}
