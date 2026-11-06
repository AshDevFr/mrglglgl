import { useTranslation } from "react-i18next";
import type { FactRow } from "@/lib/factList";

/** A day's facts as labelled lists: each kind of thing on its own row, one item per line. */
export function FactList({
  rows,
  empty,
}: {
  rows: FactRow[];
  /** Shown when nothing was recorded. */
  empty: string;
}) {
  const { t } = useTranslation();
  if (rows.length === 0) return <p className="entry">{empty}</p>;
  return (
    <dl className="fact-list" aria-label={t("facts.title")}>
      {rows.map((row) => (
        <div className="fact-row" key={row.key}>
          <dt>{row.label}</dt>
          <dd>
            <ul>
              {row.items.map((item, i) => (
                // Items can repeat (two deaths to the same creature), so the index is part of the key.
                // biome-ignore lint/suspicious/noArrayIndexKey: items may repeat
                <li key={`${i}-${item}`}>{item}</li>
              ))}
            </ul>
          </dd>
        </div>
      ))}
    </dl>
  );
}
