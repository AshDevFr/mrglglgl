import { Button, Group, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { parseDay } from "@/lib/format";
import { shiftWeek, weekDays } from "@/lib/weeks";

interface Props {
  /** The day on screen, "YYYY-MM-DD". */
  selected: string;
  /** Days that have a journal entry. */
  written: Set<string>;
  /** Today, "YYYY-MM-DD". */
  today: string;
  onSelect: (day: string) => void;
}

/** Previous and next week, a jump to today, and the seven days of the shown week. */
export function WeekNav({ selected, written, today, onSelect }: Props) {
  const { t, i18n } = useTranslation();
  const days = weekDays(selected);
  const range = t("week.range", {
    from: new Intl.DateTimeFormat(i18n.language, {
      day: "numeric",
      month: "long",
    }).format(parseDay(days[0])),
    to: new Intl.DateTimeFormat(i18n.language, { dateStyle: "long" }).format(
      parseDay(days[6]),
    ),
  });
  const weekday = new Intl.DateTimeFormat(i18n.language, { weekday: "short" });

  return (
    <Stack gap="xs" component="nav" aria-label={range}>
      <Group justify="space-between" maw="44rem" wrap="nowrap">
        <Button
          variant="subtle"
          color="gray"
          onClick={() => onSelect(shiftWeek(selected, -1, written))}
        >
          {t("week.previous")}
        </Button>
        <Text fw={500}>{range}</Text>
        <Group gap="xs" wrap="nowrap">
          <Button
            variant="subtle"
            color="gray"
            onClick={() => onSelect(shiftWeek(selected, 1, written))}
          >
            {t("week.next")}
          </Button>
          <Button variant="light" onClick={() => onSelect(today)}>
            {t("week.today")}
          </Button>
        </Group>
      </Group>
      <div className="week-strip">
        {days.map((day) => {
          const date = parseDay(day);
          const hasEntry = written.has(day);
          return (
            <button
              key={day}
              type="button"
              className={hasEntry ? "week-day written" : "week-day"}
              aria-pressed={day === selected}
              aria-current={day === today ? "date" : undefined}
              aria-label={`${new Intl.DateTimeFormat(i18n.language, { dateStyle: "full" }).format(date)}${hasEntry ? "" : `, ${t("week.noEntry")}`}`}
              disabled={!hasEntry && day !== today}
              onClick={() => onSelect(day)}
            >
              <span className="weekday">{weekday.format(date)}</span>
              <span className="date">{date.getDate()}</span>
            </button>
          );
        })}
      </div>
    </Stack>
  );
}
