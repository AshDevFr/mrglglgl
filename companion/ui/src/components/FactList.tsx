import { useState } from "react";
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

/** A row longer than this folds to its first `FOLDED` items: a busy day can list thirty places,
 * which buries every row after it. A row only a little longer stays whole, since a button that
 * hides one item saves nothing. */
const FOLD_OVER = 7;
const FOLDED = 5;

function Row({ row, linked }: { row: FactRow; linked: boolean }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const foldable = row.items.length > FOLD_OVER;
  const shown = foldable && !open ? row.items.slice(0, FOLDED) : row.items;
  return (
    <div className="fact-row">
      <dt>{row.label}</dt>
      <dd>
        <ul>
          {shown.map((item, i) => (
            // Items can repeat (two deaths to the same creature), so the index is part of the key.
            // biome-ignore lint/suspicious/noArrayIndexKey: items may repeat
            <li key={`${i}-${item.text}`}>
              <Item item={item} linked={linked} />
            </li>
          ))}
        </ul>
        {foldable && (
          <button
            type="button"
            className="fact-more"
            aria-expanded={open}
            onClick={() => setOpen((o) => !o)}
          >
            {open
              ? t("facts.fewer")
              : t("facts.more", { count: row.items.length - FOLDED })}
          </button>
        )}
      </dd>
    </div>
  );
}

/** A day's facts as labelled lists: each kind of thing on its own row, one item per line, a long
 * row folded until asked for. With Wowhead links on, each name opens Wowhead's search for it. */
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
        <Row key={row.key} row={row} linked={linked} />
      ))}
    </dl>
  );
}
