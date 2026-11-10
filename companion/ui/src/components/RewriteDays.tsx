import {
  Alert,
  Badge,
  Button,
  Checkbox,
  Group,
  Modal,
  ScrollArea,
  Select,
  Stack,
  Switch,
  Text,
} from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { rewriteDays } from "@/api/commands";
import type { NarratorInfo } from "@/bindings/NarratorInfo";
import type { Story } from "@/bindings/Story";
import {
  iconOf,
  NarratorIcon,
  narratorOption,
  narratorSection,
} from "@/components/NarratorIcon";
import { problem } from "@/i18n/problems";
import { formatDay } from "@/lib/format";
import { narratorLabel } from "@/lib/narrators";
import { weekStart } from "@/lib/weeks";

/** A day the journal has an entry for: its newest version, which is the one shown. */
interface WrittenDay {
  day: string;
  story: Story;
  /** The player wrote this version themselves. */
  edited: boolean;
}

/** The days with an entry in the journal, newest first; a day told only so far is not one. */
export function writtenDays(stories: Story[]): WrittenDay[] {
  const newest = new Map<string, Story>();
  for (const s of stories) {
    if (s.kind !== "day" || s.status !== "accepted" || s.partial) continue;
    const kept = newest.get(s.startDay);
    if (!kept || s.id > kept.id) newest.set(s.startDay, s);
  }
  return [...newest.values()]
    .sort((a, b) => b.startDay.localeCompare(a.startDay))
    .map((story) => ({
      day: story.startDay,
      story,
      edited: story.check === "edited",
    }));
}

interface Props {
  characterId: number;
  opened: boolean;
  onClose: () => void;
  stories: Story[];
  narrators: NarratorInfo[];
  /** Review mode: where the batch goes unless the player says otherwise. */
  review: boolean;
  onQueued: (count: number) => void;
}

/** Picks written days to write again in one go: by day, by week or all, with one narrator or each
 * day's usual one, into To review or straight into the journal. Days the player edited by hand
 * start unticked. */
export function RewriteDays({
  characterId,
  opened,
  onClose,
  stories,
  narrators,
  review,
  onQueued,
}: Props) {
  const { t, i18n } = useTranslation();
  const days = writtenDays(stories);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [narrator, setNarrator] = useState<string | null>(null);
  const [toReview, setToReview] = useState(review);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Opened afresh: nothing chosen, the batch going where review mode sends entries.
  const reset = () => {
    setChosen(new Set());
    setNarrator(null);
    setToReview(review);
    setError(null);
  };

  const weeks = new Map<string, WrittenDay[]>();
  for (const d of days) {
    const monday = weekStart(d.day);
    weeks.set(monday, [...(weeks.get(monday) ?? []), d]);
  }
  // Selecting many at once leaves out the days the player wrote; they are ticked one by one.
  const selectable = (list: WrittenDay[]) =>
    list.filter((d) => !d.edited).map((d) => d.day);
  const toggle = (list: string[], on: boolean) =>
    setChosen((current) => {
      const next = new Set(current);
      for (const day of list) {
        if (on) next.add(day);
        else next.delete(day);
      }
      return next;
    });
  const state = (list: WrittenDay[]) => {
    const picked = list.filter((d) => chosen.has(d.day)).length;
    return {
      checked: picked > 0 && picked === selectable(list).length,
      indeterminate: picked > 0 && picked < selectable(list).length,
    };
  };

  async function queue() {
    setBusy(true);
    setError(null);
    try {
      const list = days.map((d) => d.day).filter((d) => chosen.has(d));
      await rewriteDays(characterId, list, narrator, toReview);
      onQueued(list.length);
      onClose();
      reset();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const all = state(days);
  return (
    <Modal
      opened={opened}
      onClose={() => {
        onClose();
        reset();
      }}
      title={t("rewrite.title")}
      size="lg"
    >
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          {t("rewrite.intro")}
        </Text>
        {days.length === 0 ? (
          <Text size="sm">{t("rewrite.none")}</Text>
        ) : (
          <>
            <Checkbox
              label={t("rewrite.all", { count: selectable(days).length })}
              checked={all.checked}
              indeterminate={all.indeterminate}
              onChange={(e) =>
                toggle(selectable(days), e.currentTarget.checked)
              }
            />
            <ScrollArea.Autosize mah={360} type="auto">
              <Stack gap="sm" pr="sm">
                {[...weeks.entries()].map(([monday, list]) => {
                  const week = state(list);
                  return (
                    <Stack key={monday} gap={4}>
                      <Checkbox
                        fw={600}
                        label={t("rewrite.week", {
                          date: formatDay(monday, i18n.language, true),
                        })}
                        checked={week.checked}
                        indeterminate={week.indeterminate}
                        disabled={selectable(list).length === 0}
                        onChange={(e) =>
                          toggle(selectable(list), e.currentTarget.checked)
                        }
                      />
                      {list.map((d) => (
                        <Group key={d.day} gap="xs" wrap="nowrap" pl="lg">
                          <Checkbox
                            label={formatDay(d.day, i18n.language, true)}
                            checked={chosen.has(d.day)}
                            onChange={(e) =>
                              toggle([d.day], e.currentTarget.checked)
                            }
                          />
                          <Text
                            size="xs"
                            c="dimmed"
                            className="byline"
                            title={narratorLabel(
                              t,
                              narrators,
                              d.story.narrator,
                            )}
                          >
                            <NarratorIcon
                              icon={iconOf(narrators, d.story.narrator)}
                              size={14}
                            />
                          </Text>
                          {d.edited && (
                            <Badge size="xs" variant="outline" color="gray">
                              {t("rewrite.edited")}
                            </Badge>
                          )}
                        </Group>
                      ))}
                    </Stack>
                  );
                })}
              </Stack>
            </ScrollArea.Autosize>
            <Select
              label={t("rewrite.narrator")}
              placeholder={t("rewrite.usualNarrator")}
              data={narrators.map((n) => ({ value: n.id, label: n.name }))}
              renderOption={narratorOption(narrators)}
              leftSection={narratorSection(narrators, narrator)}
              value={narrator}
              onChange={setNarrator}
              clearable
            />
            <Switch
              label={t("rewrite.toReview")}
              description={t(
                toReview ? "rewrite.toReviewOn" : "rewrite.toReviewOff",
              )}
              checked={toReview}
              onChange={(e) => setToReview(e.currentTarget.checked)}
            />
            {error && (
              <Alert color="red">
                {problem(t, "rewrite.failed", { message: error })}
              </Alert>
            )}
            <Group justify="space-between">
              <Text size="sm" c="dimmed">
                {chosen.size > 0 && t("rewrite.time", { count: chosen.size })}
              </Text>
              <Button
                disabled={chosen.size === 0}
                loading={busy}
                onClick={() => void queue()}
              >
                {t("rewrite.go", { count: chosen.size })}
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Modal>
  );
}
