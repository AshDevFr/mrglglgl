/**
 * The app's own little murloc, drawn for this project: a round, damp creature with a
 * three-spined crest. Shown only in empty and loading states.
 */
export function MurlocMark({ size = 96 }: { size?: number }) {
  return (
    <svg
      className="murloc-mark"
      width={size}
      height={size}
      viewBox="0 0 120 120"
      aria-hidden="true"
      focusable="false"
    >
      <g
        className="bob"
        fill="none"
        stroke="currentColor"
        strokeWidth={3}
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        {/* Crest */}
        <path
          d="M44 30 Q48 12 54 26 Q60 8 66 26 Q72 12 76 30"
          fill="currentColor"
          fillOpacity={0.18}
        />
        {/* Head and body in one round shape */}
        <path
          d="M30 62 C30 38 46 28 60 28 C74 28 90 38 90 62 C90 86 78 100 60 100 C42 100 30 86 30 62 Z"
          fill="currentColor"
          fillOpacity={0.12}
        />
        {/* Eyes */}
        <circle cx={48} cy={52} r={8} fill="var(--page, #fff)" />
        <circle cx={72} cy={52} r={8} fill="var(--page, #fff)" />
        <circle cx={50} cy={53} r={3.2} fill="currentColor" stroke="none" />
        <circle cx={70} cy={53} r={3.2} fill="currentColor" stroke="none" />
        {/* A wide, hopeful mouth */}
        <path d="M44 72 Q60 86 76 72" />
        <path
          d="M51 76 L52 80 M60 78.5 L60 82.5 M69 76 L68 80"
          strokeWidth={2}
        />
        {/* Flippers */}
        <path d="M31 70 Q20 74 22 84" />
        <path d="M89 70 Q100 74 98 84" />
        {/* A drip, because it is always a little wet */}
        <path
          d="M84 40 Q86 44 84 46 Q82 44 84 40 Z"
          fill="currentColor"
          fillOpacity={0.5}
          strokeWidth={1.5}
        />
      </g>
    </svg>
  );
}
