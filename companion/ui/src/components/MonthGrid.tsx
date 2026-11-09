import {
  ActionIcon,
  Group,
  SimpleGrid,
  Stack,
  Text,
  UnstyledButton,
} from "@mantine/core";
import { useTranslation } from "react-i18next";
import { formatDay, parseDay } from "@/lib/format";
import { dayKey } from "@/lib/weeks";

interface Props {
  /** The month shown, "YYYY-MM". */
  month: string;
  selected: string;
  /** Days with something to read. */
  written: Set<string>;
  today: string;
  /** The earliest and latest months there is anything to show. */
  first: string;
  last: string;
  onMonth: (month: string) => void;
  onSelect: (day: string) => void;
}

const WEEK = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/** "2026-11" shifted by `by` months. */
export function shiftMonth(month: string, by: number): string {
  const [year, m] = month.split("-").map(Number);
  const date = new Date(year, m - 1 + by, 1);
  return dayKey(date).slice(0, 7);
}

/** A month as a calendar, Monday first: written days inked and openable, as in the week strip,
 * today openable too, every other day shown but not openable. */
export function MonthGrid({
  month,
  selected,
  written,
  today,
  first,
  last,
  onMonth,
  onSelect,
}: Props) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const start = parseDay(`${month}-01`);
  // Blank cells before the first: Monday is the week's first day.
  const blanks = (start.getDay() + 6) % 7;
  const length = new Date(
    start.getFullYear(),
    start.getMonth() + 1,
    0,
  ).getDate();
  const days = Array.from(
    { length },
    (_, i) => `${month}-${String(i + 1).padStart(2, "0")}`,
  );
  const weekday = new Intl.DateTimeFormat(lang, { weekday: "narrow" });
  // The columns, named from a week that starts on a Monday.
  const columns = WEEK.map((id, i) => ({
    id,
    name: weekday.format(new Date(2026, 10, 2 + i)),
  }));
  // The previous month's last days stand empty before the first: keyed by their own dates.
  const lead = Array.from({ length: blanks }, (_, i) =>
    dayKey(new Date(start.getFullYear(), start.getMonth(), i - blanks + 1)),
  );
  const label = new Intl.DateTimeFormat(lang, {
    month: "long",
    year: "numeric",
  }).format(start);

  return (
    <Stack gap="xs" aria-label={label}>
      <Group justify="space-between" wrap="nowrap">
        <ActionIcon
          variant="subtle"
          aria-label={t("week.previousMonth")}
          disabled={month <= first}
          onClick={() => onMonth(shiftMonth(month, -1))}
        >
          ‹
        </ActionIcon>
        <Text fw={600}>{label}</Text>
        <ActionIcon
          variant="subtle"
          aria-label={t("week.nextMonth")}
          disabled={month >= last}
          onClick={() => onMonth(shiftMonth(month, 1))}
        >
          ›
        </ActionIcon>
      </Group>
      <SimpleGrid cols={7} spacing={2} verticalSpacing={2}>
        {columns.map((column) => (
          <Text
            key={column.id}
            size="xs"
            c="dimmed"
            ta="center"
            aria-hidden="true"
          >
            {column.name}
          </Text>
        ))}
        {lead.map((day) => (
          <span key={day} />
        ))}
        {days.map((day) => {
          const open = written.has(day) || day === today;
          return (
            <UnstyledButton
              key={day}
              className="month-day"
              data-written={written.has(day) || undefined}
              aria-current={day === today ? "date" : undefined}
              aria-pressed={day === selected}
              aria-label={formatDay(day, lang, true)}
              disabled={!open}
              onClick={() => onSelect(day)}
            >
              {Number(day.slice(8))}
            </UnstyledButton>
          );
        })}
      </SimpleGrid>
    </Stack>
  );
}
