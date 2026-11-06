import fishing from "@/assets/murloc/fishing.webp";
import scribe from "@/assets/murloc/scribe.webp";
import sleep from "@/assets/murloc/sleep.webp";
import wave from "@/assets/murloc/wave.webp";

const ART = { scribe, wave, fishing, sleep } as const;
export type MurlocPose = keyof typeof ART;

/** The baby murloc, drawn: decoration only, so it is hidden from screen readers. */
export function MurlocArt({ pose, size }: { pose: MurlocPose; size: number }) {
  return (
    <img
      src={ART[pose]}
      alt=""
      aria-hidden="true"
      className="murloc-art"
      style={{ width: size, height: size, objectFit: "contain", flexShrink: 0 }}
    />
  );
}
