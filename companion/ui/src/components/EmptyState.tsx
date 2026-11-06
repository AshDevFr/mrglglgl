import { Group, Stack, Text } from "@mantine/core";
import type { ReactNode } from "react";
import { MurlocMark } from "./MurlocMark";

/** The murloc, and what to do next. Used only where there is nothing (yet) to read. */
export function EmptyState({
  title,
  children,
}: {
  title: string;
  children?: ReactNode;
}) {
  return (
    <Group gap="lg" align="center" wrap="nowrap" maw="40rem" py="xl">
      <MurlocMark />
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
