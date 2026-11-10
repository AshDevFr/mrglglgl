import { Group, Loader, Text } from "@mantine/core";

/** Says an entry is being written, with a spinner, in the accent colour so it stands out from
 * the notes around it. Announced once; the words that follow are not read out as they arrive. */
export function WritingNow({ label }: { label: string }) {
  return (
    <Group gap="xs" wrap="nowrap" className="writing-now" role="status">
      <Loader size="sm" color="var(--stamp)" type="dots" aria-hidden />
      <Text fw={600} c="var(--stamp)">
        {label}
      </Text>
    </Group>
  );
}
