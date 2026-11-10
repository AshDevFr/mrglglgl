import {
  Button,
  Group,
  Popover,
  Stack,
  Text,
  UnstyledButton,
} from "@mantine/core";
import { type KeyboardEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { parseDay } from "@/lib/format";
import { shiftWeek, weekDays, weekStart } from "@/lib/weeks";
import { MonthGrid } from "./MonthGrid";

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
  const [picking, setPicking] = useState(false);
  const [month, setMonth] = useState(selected.slice(0, 7));
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
  // Nothing is written ahead of today: the week holding it is as far as there is to go.
  const inThisWeek = weekStart(selected) >= weekStart(today);
  // Before the first written day there is nothing to read.
  const first = [...written].sort()[0];
  const atFirstWeek =
    first !== undefined && weekStart(selected) <= weekStart(first);

  // Arrow keys step through the days that can be opened; Home and End jump to the ends.
  function onDayKey(e: KeyboardEvent<HTMLButtonElement>) {
    const open = days.filter((d) => written.has(d) || d === today);
    const at = open.indexOf(selected);
    const target =
      e.key === "ArrowRight"
        ? open[at + 1]
        : e.key === "ArrowLeft"
          ? open[at - 1]
          : e.key === "Home"
            ? open[0]
            : e.key === "End"
              ? open.at(-1)
              : undefined;
    if (!target) return;
    e.preventDefault();
    onSelect(target);
    const strip = e.currentTarget.parentElement;
    // The selected day keeps the focus once the strip redraws.
    requestAnimationFrame(() =>
      strip
        ?.querySelector<HTMLButtonElement>(`[data-day="${target}"]`)
        ?.focus(),
    );
  }

  return (
    <Stack gap="xs" component="nav" aria-label={range} className="week-nav">
      {/* In a narrow window the range takes a row of its own, above the buttons. */}
      <div className="week-bar">
        <Button
          variant="subtle"
          color="gray"
          disabled={atFirstWeek}
          onClick={() => onSelect(shiftWeek(selected, -1, written))}
        >
          {t("week.previous")}
        </Button>
        {/* The range opens a month calendar, to reach any day without stepping week by week. */}
        <div className="week-bar-range">
          <Popover
            opened={picking}
            onChange={setPicking}
            position="bottom"
            withArrow
            shadow="md"
            trapFocus
          >
            <Popover.Target>
              <UnstyledButton
                className="week-range"
                aria-haspopup="dialog"
                aria-expanded={picking}
                onClick={() => {
                  setMonth(selected.slice(0, 7));
                  setPicking((open) => !open);
                }}
              >
                <Text fw={500}>{range} ▾</Text>
              </UnstyledButton>
            </Popover.Target>
            <Popover.Dropdown role="dialog">
              <MonthGrid
                month={month}
                selected={selected}
                written={written}
                today={today}
                first={(first ?? today).slice(0, 7)}
                last={today.slice(0, 7)}
                onMonth={setMonth}
                onSelect={(day) => {
                  setPicking(false);
                  onSelect(day);
                }}
              />
            </Popover.Dropdown>
          </Popover>
        </div>
        <Group gap="xs" wrap="nowrap">
          <Button
            variant="subtle"
            color="gray"
            disabled={inThisWeek}
            onClick={() => onSelect(shiftWeek(selected, 1, written))}
          >
            {t("week.next")}
          </Button>
          <Button
            variant="light"
            disabled={selected === today}
            onClick={() => onSelect(today)}
          >
            {t("week.today")}
          </Button>
        </Group>
      </div>
      <div className="week-strip">
        {days.map((day) => {
          const date = parseDay(day);
          const hasEntry = written.has(day);
          return (
            <button
              key={day}
              type="button"
              className={hasEntry ? "week-day written" : "week-day"}
              data-day={day}
              aria-pressed={day === selected}
              aria-current={day === today ? "date" : undefined}
              aria-label={`${new Intl.DateTimeFormat(i18n.language, { dateStyle: "full" }).format(date)}${hasEntry ? "" : `, ${t("week.noEntry")}`}`}
              disabled={!hasEntry && day !== today}
              onClick={() => onSelect(day)}
              onKeyDown={onDayKey}
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
