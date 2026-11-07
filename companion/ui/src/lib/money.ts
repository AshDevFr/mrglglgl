import type { TFunction } from "i18next";
import type { Day } from "@/bindings/Day";

/** Copper as the game shows it, "3g 20s 5c", leaving out empty parts. The unit letters come from
 * the language: French writes "po", "pa", "pc". */
export function gold(copper: number, t: TFunction): string {
  const g = Math.floor(copper / 10000);
  const s = Math.floor(copper / 100) % 100;
  const c = copper % 100;
  const parts: string[] = [];
  if (g > 0) parts.push(t("money.gold", { n: g }));
  if (s > 0) parts.push(t("money.silver", { n: s }));
  if (c > 0 || parts.length === 0) parts.push(t("money.copper", { n: c }));
  return parts.join(" ");
}

/** One source of money and how much, in copper. */
export interface MoneyLine {
  source: string;
  copper: number;
}

export interface MoneyFacts {
  /** Earned, largest first. */
  earned: MoneyLine[];
  /** Spent, largest first. */
  spent: MoneyLine[];
  /** The day's first and last known balance. */
  balStart: number | null;
  balEnd: number | null;
  /** Auction house events in order: `act` is listed, sold, won, outbid, expired or cancelled. */
  auctions: { act: string; name: string | null }[];
}

const lines = (side: Record<string, number | undefined>): MoneyLine[] =>
  Object.entries(side)
    .filter((e): e is [string, number] => (e[1] ?? 0) > 0)
    .map(([source, copper]) => ({ source, copper }))
    .sort((a, b) => b.copper - a.copper || a.source.localeCompare(b.source));

/** Money and auctions: the day's own records while it has them, the digest otherwise. A compacted
 * day keeps its money, its sessions and its auction events. */
export function moneyFacts(day: Day): MoneyFacts {
  const d = day.digest;
  const own = day.money ?? { inc: {}, out: {} };
  const empty =
    Object.keys(own.inc).length === 0 && Object.keys(own.out).length === 0;
  const money = empty && d ? d.money : own;
  let balStart: number | null = null;
  let balEnd: number | null = null;
  const auctions: MoneyFacts["auctions"] = [];
  for (const event of day.events ?? []) {
    if (event.balance !== null && event.balance !== undefined) {
      balStart ??= event.balance;
      balEnd = event.balance;
    }
    if (event.kind.kind === "auction" && event.kind.act)
      auctions.push({ act: event.kind.act, name: event.kind.name });
  }
  if (day.events === null && d) {
    for (const a of d.auctions)
      if (a.act) auctions.push({ act: a.act, name: a.name });
  }
  return {
    earned: lines(money.inc),
    spent: lines(money.out),
    balStart: balStart ?? d?.balStart ?? null,
    balEnd: balEnd ?? d?.balEnd ?? null,
    auctions,
  };
}
