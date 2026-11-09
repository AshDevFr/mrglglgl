import { Stack, Text, Tooltip, UnstyledButton } from "@mantine/core";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import type { Award } from "@/bindings/Award";
import { formatDayShort } from "@/lib/format";

interface Props {
  award: Award;
  size?: number;
  /** Degrees; stamps are never pressed perfectly straight. */
  tilt?: number;
  /** Shows the stamp large, with its name and what it is for, on hover. Off where nobody can
   * hover, such as an exported page. */
  zoom?: boolean;
}

/** How large the stamp shows in its tooltip, where its rim can be read. */
const ZOOMED_SIZE = 160;

/** A milestone as a round rubber stamp: its name around the rim, the day in the middle. Small
 * on the page, so hovering it shows it large with its name and description. */
export function InkStamp({ award, size = 72, tilt = -6, zoom = true }: Props) {
  const { t } = useTranslation();
  // Without the tooltip, the browser's own hover text says what the stamp is for.
  const mark = (
    <StampMark award={award} size={size} tilt={tilt} titled={!zoom} />
  );
  if (!zoom) return mark;
  return (
    <Tooltip
      events={{ hover: true, focus: true, touch: false }}
      withArrow
      multiline
      w={220}
      // On the page's own paper and ink, in every theme, not the library's grey.
      classNames={{ tooltip: "stamp-tooltip", arrow: "stamp-tooltip-arrow" }}
      label={
        <Stack gap={4} align="center" py={4}>
          <StampMark award={award} size={ZOOMED_SIZE} tilt={0} titled={false} />
          <Text fw={700} size="sm" ta="center">
            {t(`milestones.${award.id}.name`)}
          </Text>
          <Text size="xs" ta="center">
            {t(`milestones.${award.id}.description`)}
          </Text>
        </Stack>
      }
    >
      {/* The tooltip needs an element that takes a ref; the stamp is an SVG. */}
      {/* A button, so the tooltip opens from the keyboard as well as on hover. */}
      <UnstyledButton style={{ display: "inline-flex" }}>{mark}</UnstyledButton>
    </Tooltip>
  );
}

function StampMark({
  award,
  size,
  tilt,
  titled,
}: {
  award: Award;
  size: number;
  tilt: number;
  /** Carries the description as the browser's hover text. */
  titled: boolean;
}) {
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
      {titled && <title>{t(`milestones.${award.id}.description`)}</title>}
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
