import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { onWriting } from "@/api/commands";
import { applyEvent, type LiveMap } from "./writing";

/** Streams what the background writer is doing for a character, and refreshes the entries and
 * the queue when it finishes something. */
export function useWriting(characterId: number): LiveMap {
  const queryClient = useQueryClient();
  const [live, setLive] = useState<LiveMap>({});
  useEffect(() => {
    setLive({});
    let stop: (() => void) | undefined;
    let cancelled = false;
    onWriting((event) => {
      setLive((map) => applyEvent(map, characterId, event));
      if (event.type === "finished" || event.type === "started") {
        void queryClient.invalidateQueries({ queryKey: ["writing"] });
        if (event.characterId === characterId) {
          for (const queryKey of [
            ["stories", characterId],
            // Every To review list: this character's and the one for every character.
            ["review"],
            ["characters"],
          ]) {
            void queryClient.invalidateQueries({ queryKey });
          }
        }
      }
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [characterId, queryClient]);
  return live;
}
