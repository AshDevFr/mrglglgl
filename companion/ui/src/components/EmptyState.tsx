import { Group, Stack, Text } from "@mantine/core";
import type { ReactNode } from "react";
import { MurlocArt, type MurlocPose } from "./MurlocArt";
import { MurlocMark } from "./MurlocMark";

/** The murloc, and what to do next. Used only where there is nothing (yet) to read. */
export function EmptyState({
  title,
  children,
  art,
}: {
  title: string;
  children?: ReactNode;
  /** A drawing of the baby murloc; without one, the small mark (as while loading). */
  art?: MurlocPose;
}) {
  return (
    <Group gap="lg" align="center" wrap="nowrap" maw="40rem" py="xl">
      {art ? <MurlocArt pose={art} size={132} /> : <MurlocMark />}
      <Stack gap={4}>
        <Text
          className="entry"
          style={{ fontFamily: "var(--reading-font)", fontSize: "1.15rem" }}
        >
          {title}
        </Text>
        {children && (
          <Text size="sm" c="dimmed">
            {children}
          </Text>
        )}
      </Stack>
    </Group>
  );
}
