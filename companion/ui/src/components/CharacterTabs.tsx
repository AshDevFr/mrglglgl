import { Badge, Group, UnstyledButton } from "@mantine/core";
import { useTranslation } from "react-i18next";

export type CharacterTab = "journal" | "review" | "milestones" | "settings";

interface Props {
  current: CharacterTab;
  /** Days and weeks with a draft waiting, shown on the To review tab. */
  toReview: number;
  onSelect: (tab: CharacterTab) => void;
}

/** A character's pages, as tabs under the name: where the reader is, and where else to go. */
export function CharacterTabs({ current, toReview, onSelect }: Props) {
  const { t } = useTranslation();
  const tab = (id: CharacterTab, label: string, count = 0) => (
    <UnstyledButton
      key={id}
      className="character-tab"
      aria-current={current === id ? "page" : undefined}
      onClick={() => onSelect(id)}
    >
      {label}
      {count > 0 && (
        <Badge size="sm" variant="light" ml={6} aria-label={String(count)}>
          {count}
        </Badge>
      )}
    </UnstyledButton>
  );
  return (
    <nav aria-label={t("nav.characterPages")} className="character-tabs">
      <Group gap="lg" wrap="wrap">
        {tab("journal", t("nav.journal"))}
        {tab("review", t("nav.review"), toReview)}
        {tab("milestones", t("nav.milestones"))}
        {tab("settings", t("nav.characterSettings"))}
      </Group>
    </nav>
  );
}
