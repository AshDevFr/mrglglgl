import {
  Alert,
  Button,
  Group,
  Radio,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  cancelPreview,
  characterJournal,
  listCharacters,
  onNarratorPreview,
  previewNarrator,
  saveNarrator,
} from "@/api/commands";
import type { Invalid } from "@/bindings/Invalid";
import type { Narrator } from "@/bindings/Narrator";
import type { PointOfView } from "@/bindings/PointOfView";
import type { PreviewDay } from "@/bindings/PreviewDay";
import type { PreviewResult } from "@/bindings/PreviewResult";
import { problem } from "@/i18n/problems";
import { formatDay } from "@/lib/format";
import { daysNewestFirst, displayName } from "@/lib/journal";
import { dayKey } from "@/lib/weeks";
import { EntryText } from "./EntryPanel";

/** The day patterns a narrator can have a hint for, in the order the form lists them. */
const PATTERNS = [
  "only_fished",
  "many_deaths",
  "big_level_jump",
  "new_zone",
  "rare_loot",
  "murloc_heavy",
  "murloc_death",
  "nothing_happened",
  "final_death",
];

const SAMPLE = "sample";

interface Props {
  initial: Narrator;
  onSaved?: (narrator: Narrator) => void;
}

/** A player's narrator: the voice up front, the rest folded under More, and a preview. */
export function NarratorEditor({ initial, onSaved }: Props) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<Narrator>(initial);
  const [problems, setProblems] = useState<Invalid[]>([]);
  const [status, setStatus] = useState<"saved" | "fix" | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const set = (patch: Partial<Narrator>) => {
    setDraft((d) => ({ ...d, ...patch }));
    setStatus(null);
  };
  const setExample = (patch: Partial<Narrator["example"]>) =>
    set({ example: { ...draft.example, ...patch } });
  const setHint = (key: string, value: string) =>
    set({ hints: { ...draft.hints, [key]: value } });
  const error = (field: string) => {
    const found = problems.find((p) => p.field === field);
    return found ? t(`narrators.problem.${found.problem}`) : undefined;
  };

  async function save() {
    setFailure(null);
    try {
      const result = await saveNarrator(draft);
      if (result.kind === "saved") {
        setProblems([]);
        setDraft(result.narrator);
        setStatus("saved");
        await queryClient.invalidateQueries({ queryKey: ["narrators"] });
        onSaved?.(result.narrator);
      } else {
        setProblems(result.problems);
        setStatus("fix");
      }
    } catch (e) {
      setFailure(String(e));
    }
  }

  const text = (
    field: string,
    label: string,
    value: string,
    onChange: (v: string) => void,
    description?: string,
  ) => (
    <Textarea
      label={label}
      description={description}
      value={value}
      onChange={(e) => onChange(e.currentTarget.value)}
      error={error(field)}
      autosize
      minRows={2}
    />
  );

  return (
    <Stack gap="lg">
      <Stack gap="sm">
        <TextInput
          label={t("narrators.name")}
          value={draft.name}
          onChange={(e) => set({ name: e.currentTarget.value })}
          error={error("name")}
        />
        {text(
          "persona",
          t("narrators.persona"),
          draft.persona,
          (persona) => set({ persona }),
          t("narrators.personaHelp"),
        )}
        <Radio.Group
          label={t("narrators.pointOfView")}
          description={t("narrators.pointOfViewHelp")}
          value={draft.point_of_view}
          onChange={(v) => set({ point_of_view: v as PointOfView })}
        >
          <Group gap="md" mt={4}>
            <Radio value="adventurer" label={t("narrators.asAdventurer")} />
            <Radio value="observer" label={t("narrators.aboutAdventurer")} />
          </Group>
        </Radio.Group>
        {text(
          "voice",
          t("narrators.voice"),
          draft.voice,
          (voice) => set({ voice }),
          t("narrators.voiceHelp"),
        )}
        {text(
          "example.entry",
          t("narrators.exampleEntry"),
          draft.example.entry,
          (entry) => setExample({ entry }),
          t("narrators.exampleEntryHelp"),
        )}
        <details className="facts">
          <summary>{t("narrators.more")}</summary>
          <Stack gap="sm" mt="sm">
            {text(
              "murloc",
              t("narrators.murloc"),
              draft.murloc,
              (murloc) => set({ murloc }),
              t("narrators.murlocHelp"),
            )}
            {text(
              "margin_note",
              t("narrators.marginNote"),
              draft.margin_note,
              (margin_note) => set({ margin_note }),
              t("narrators.marginNoteHelp"),
            )}
            {text(
              "example.notes",
              t("narrators.exampleNotes"),
              draft.example.notes,
              (notes) => setExample({ notes }),
              t("narrators.exampleNotesHelp"),
            )}
            {text(
              "example.margin_note",
              t("narrators.exampleMarginNote"),
              draft.example.margin_note,
              (margin_note) => setExample({ margin_note }),
            )}
            <Title order={4} size="h5">
              {t("narrators.hints")}
            </Title>
            <Text size="sm" c="dimmed">
              {t("narrators.hintsHelp")}
            </Text>
            {PATTERNS.map((key) =>
              text(
                `hints.${key}`,
                t(`narrators.hint.${key}`),
                draft.hints[key] ?? "",
                (v) => setHint(key, v),
              ),
            )}
          </Stack>
        </details>
      </Stack>
      {status === "fix" && <Alert color="yellow">{t("narrators.fix")}</Alert>}
      {status === "saved" && (
        <Alert color="green">{t("narrators.saved")}</Alert>
      )}
      {failure && (
        <Alert color="red">
          {problem(t, "narrators.fileFailed", { message: failure })}
        </Alert>
      )}
      <Group>
        <Button onClick={() => void save()}>{t("narrators.save")}</Button>
      </Group>
      <NarratorPreview narrator={draft} />
    </Stack>
  );
}

/** Writes one day with the narrator as it stands in the form; nothing is stored. */
export function NarratorPreview({ narrator }: { narrator: Narrator }) {
  const { t, i18n } = useTranslation();
  const [characterId, setCharacterId] = useState<string | null>(null);
  const [day, setDay] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [streamed, setStreamed] = useState("");
  const [result, setResult] = useState<PreviewResult | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const characters = useQuery({
    queryKey: ["characters"],
    queryFn: listCharacters,
  });
  const chosen =
    characterId ?? (characters.data?.[0]?.character.id.toString() || null);
  const journal = useQuery({
    queryKey: ["journal", Number(chosen)],
    queryFn: () => characterJournal(Number(chosen)),
    enabled: chosen !== null,
  });
  const today = dayKey(new Date());
  const days = journal.data
    ? daysNewestFirst(journal.data).filter((d) => d < today)
    : [];
  const pickedDay = day ?? days[0] ?? SAMPLE;

  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    onNarratorPreview((event) => {
      if (event.type === "chunk") setStreamed((s) => s + event.text);
      else setStreamed("");
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  async function run() {
    const target: PreviewDay =
      chosen === null || pickedDay === SAMPLE
        ? { kind: "sample" }
        : { kind: "character", characterId: Number(chosen), day: pickedDay };
    setRunning(true);
    setStreamed("");
    setResult(null);
    setFailure(null);
    try {
      setResult(await previewNarrator(narrator, target));
    } catch (e) {
      setFailure(String(e));
    } finally {
      setRunning(false);
    }
  }

  return (
    <Stack gap="sm">
      <Title order={3} size="h4">
        {t("narrators.previewTitle")}
      </Title>
      <Text size="sm" c="dimmed">
        {t("narrators.previewHelp")}
      </Text>
      <Group gap="xs" wrap="wrap">
        {(characters.data?.length ?? 0) > 0 && (
          <Select
            label={t("narrators.previewCharacter")}
            data={(characters.data ?? []).map((c) => ({
              value: String(c.character.id),
              label: displayName(c.character),
            }))}
            value={chosen}
            onChange={(v) => {
              setCharacterId(v);
              setDay(null);
            }}
            allowDeselect={false}
            size="xs"
            w={200}
          />
        )}
        {(characters.data?.length ?? 0) > 0 && (
          <Select
            label={t("narrators.previewDay")}
            data={[
              ...days.map((d) => ({
                value: d,
                label: formatDay(d, i18n.language),
              })),
              { value: SAMPLE, label: t("narrators.sampleDay") },
            ]}
            value={pickedDay}
            onChange={setDay}
            allowDeselect={false}
            size="xs"
            w={200}
          />
        )}
      </Group>
      <Group gap="xs">
        <Button
          variant="light"
          loading={running}
          onClick={() => void run()}
          disabled={running}
        >
          {t("narrators.preview")}
        </Button>
        {running && (
          <Button
            variant="subtle"
            color="gray"
            onClick={() => void cancelPreview()}
          >
            {t("narrators.previewCancel")}
          </Button>
        )}
      </Group>
      {running && (
        <div aria-busy="true">
          <Text className="writing-status" role="status">
            {t("narrators.previewing")}
          </Text>
          {streamed && <p className="entry streaming">{streamed}</p>}
        </div>
      )}
      {result?.outcome === "written" && (
        <div className="journal-page">
          <EntryText text={result.text} />
          {result.reason && (
            <Text size="sm" className="note">
              {problem(t, "narrators.previewSetAside", {
                reason: result.reason,
              })}
            </Text>
          )}
        </div>
      )}
      {result?.outcome === "cancelled" && (
        <Text size="sm" className="note">
          {t("narrators.previewCancelled")}
        </Text>
      )}
      {result?.outcome === "timedOut" && (
        <Text size="sm" className="note">
          {t("narrators.previewTimedOut")}
        </Text>
      )}
      {failure && (
        <Alert color="red">
          {problem(t, "narrators.previewFailed", { message: failure })}
        </Alert>
      )}
    </Stack>
  );
}
