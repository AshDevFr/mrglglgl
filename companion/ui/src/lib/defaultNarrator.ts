import { useQueries, useQuery } from "@tanstack/react-query";
import { characterWriting, getSettings } from "@/api/commands";

/** The narrator each character writes with when none is picked for an entry: its own, set on its
 * writing tab, or the one in Settings. Keyed by character; missing until both are loaded. */
export function useDefaultNarrators(ids: number[]): Record<number, string> {
  const settings = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const own = useQueries({
    queries: ids.map((id) => ({
      queryKey: ["characterWriting", id],
      queryFn: () => characterWriting(id),
    })),
  });
  const general = settings.data?.settings.writing.narrator;
  const out: Record<number, string> = {};
  ids.forEach((id, i) => {
    const mine = own[i]?.data;
    if (!mine || !general) return;
    out[id] = mine.narrator ?? general;
  });
  return out;
}
