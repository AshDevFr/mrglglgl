import { useTranslation } from "react-i18next";
import { openLink } from "@/api/commands";
import type { FactItem, FactRow } from "@/lib/factList";
import { useWowheadLinks, wowheadSearch } from "@/lib/wowhead";

function Item({ item, linked }: { item: FactItem; linked: boolean }) {
  if (!linked || !item.name) return <>{item.text}</>;
  const url = wowheadSearch(item.name);
  return (
    <a
      href={url}
      onClick={(e) => {
        // The app's window never navigates away: the link opens in the browser.
        e.preventDefault();
        void openLink(url);
      }}
    >
      {item.text}
    </a>
  );
}

/** A day's facts as labelled lists: each kind of thing on its own row, one item per line. With
 * Wowhead links on, each name opens Wowhead's search for it. */
export function FactList({
  rows,
  empty,
}: {
  rows: FactRow[];
  /** Shown when nothing was recorded. */
  empty: string;
}) {
  const { t } = useTranslation();
  const linked = useWowheadLinks((s) => s.on);
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
                <li key={`${i}-${item.text}`}>
                  <Item item={item} linked={linked} />
                </li>
              ))}
            </ul>
          </dd>
        </div>
      ))}
    </dl>
  );
}
