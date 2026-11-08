import { useId } from "react";
import { useTranslation } from "react-i18next";
import type { Award } from "@/bindings/Award";
import { formatDayShort } from "@/lib/format";

interface Props {
  award: Award;
  size?: number;
  /** Degrees; stamps are never pressed perfectly straight. */
  tilt?: number;
}

/** A milestone as a round rubber stamp: its name around the rim, the day in the middle. */
export function InkStamp({ award, size = 84, tilt = -6 }: Props) {
  const { t, i18n } = useTranslation();
  const id = useId().replace(/:/g, "");
  const name = t(`milestones.${award.id}.name`);
  return (
    <svg
      className="ink-stamp"
      width={size}
      height={size}
      viewBox="0 0 120 120"
      role="img"
      aria-label={`${name}: ${t(`milestones.${award.id}.description`)}`}
      style={{ transform: `rotate(${tilt}deg)` }}
    >
      <title>{t(`milestones.${award.id}.description`)}</title>
      <defs>
        {/* Uneven ink: a slight wobble of every edge. */}
        <filter id={`rough-${id}`}>
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.9"
            numOctaves={2}
            seed={4}
          />
          <feDisplacementMap in="SourceGraphic" scale={2.2} />
        </filter>
        <path
          id={`rim-${id}`}
          d="M 60,60 m -42,0 a 42,42 0 1,1 84,0 a 42,42 0 1,1 -84,0"
        />
      </defs>
      <g filter={`url(#rough-${id})`} fill="none" stroke="currentColor">
        <circle cx={60} cy={60} r={56} strokeWidth={3} />
        <circle cx={60} cy={60} r={33} strokeWidth={1.2} />
        <text
          fill="currentColor"
          stroke="none"
          fontSize={11}
          fontWeight={700}
          letterSpacing={1.2}
          fontFamily="var(--reading-font)"
        >
          <textPath href={`#rim-${id}`} startOffset="50%" textAnchor="middle">
            {name.toUpperCase()}
          </textPath>
        </text>
        <text
          x={60}
          y={64}
          textAnchor="middle"
          fill="currentColor"
          stroke="none"
          fontSize={12}
          fontFamily="var(--reading-font)"
        >
          {formatDayShort(award.day, i18n.language)}
        </text>
      </g>
    </svg>
  );
}
