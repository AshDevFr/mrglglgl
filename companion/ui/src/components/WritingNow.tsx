import { Group, Loader, Text } from "@mantine/core";
import { NarratorIcon } from "@/components/NarratorIcon";

/** Says an entry is being written, with a spinner and the narrator's glyph, in the accent colour so it stands out from
 * the notes around it. Announced once; the words that follow are not read out as they arrive. */
export function WritingNow({
  label,
  icon,
}: {
  label: string;
  /** The writing narrator's glyph, once known. */
  icon?: string;
}) {
  return (
    <Group gap="xs" wrap="nowrap" className="writing-now" role="status">
      <Loader size="sm" color="var(--stamp)" type="dots" aria-hidden />
      {icon && (
        <Text c="var(--stamp)" lh={0}>
          <NarratorIcon icon={icon} size={20} />
        </Text>
      )}
      <Text fw={600} c="var(--stamp)">
        {label}
      </Text>
    </Group>
  );
}
