import { List, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { DayFacts, KillLine } from "@/lib/journal";

interface Props {
  facts: DayFacts;
}

/** A day's facts as plain sentences. The journal prose comes later; this is the record. */
export function DayFactsList({ facts }: Props) {
  const { t } = useTranslation();
  const join = (items: string[]) => items.join(t("list.separator"));
  const killName = (k: KillLine) =>
    k.name ?? t("day.unknownCreature", { id: k.npcId ?? "?" });

  const lines: string[] = [];
  if (facts.leveledUp && facts.levelTo !== null) {
    lines.push(t("day.levelUp", { to: facts.levelTo }));
  }
  if (facts.zones.length > 0)
    lines.push(t("day.zones", { list: join(facts.zones) }));
  if (facts.discovered.length > 0) {
    lines.push(t("day.discovered", { list: join(facts.discovered) }));
  }
  if (facts.accepted.length > 0) {
    lines.push(t("day.accepted", { list: join(facts.accepted) }));
  }
  if (facts.turnedIn.length > 0) {
    lines.push(t("day.turnedIn", { list: join(facts.turnedIn) }));
  }
  if (facts.kills.length > 0) {
    const list = facts.kills.map((k) =>
      t("day.killEntry", { name: killName(k), count: k.count }),
    );
    let line = t("day.killed", { list: join(list) });
    if (facts.murlocs > 0)
      line += ` (${t("day.murlocs", { count: facts.murlocs })})`;
    lines.push(line);
  }
  if (facts.anonKills > 0)
    lines.push(t("day.anonKills", { count: facts.anonKills }));
  if (facts.notable.length > 0) {
    const list = facts.notable.map(
      (k) => `${killName(k)} (${t(`day.classification.${k.classification}`)})`,
    );
    lines.push(t("day.notable", { list: join(list) }));
  }
  for (const death of facts.deaths) {
    if (death.environment) {
      lines.push(
        t(`day.deathEnvironment.${death.environment}`, {
          defaultValue: t("day.deathUnknown"),
        }),
      );
    } else if (death.killer) {
      lines.push(t("day.deathBy", { name: death.killer }));
    } else {
      lines.push(t("day.deathUnknown"));
    }
  }
  if (facts.fish > 0) {
    let line = t("day.fish", { count: facts.fish });
    if (facts.boots > 0)
      line += `, ${t("day.fishBoots", { count: facts.boots })}`;
    lines.push(line);
    const catches = facts.catches.map((c) =>
      t("day.killEntry", { name: c.name ?? `#${c.itemId}`, count: c.count }),
    );
    if (catches.length > 0)
      lines.push(t("day.fishList", { list: join(catches) }));
  }
  if (facts.loot.length > 0)
    lines.push(t("day.loot", { list: join(facts.loot) }));
  if (facts.companions.length > 0) {
    lines.push(t("day.companions", { list: join(facts.companions) }));
  }
  if (facts.bosses.length > 0)
    lines.push(t("day.bosses", { list: join(facts.bosses) }));
  if (facts.screenshots > 0) {
    lines.push(t("day.screenshots", { count: facts.screenshots }));
  }
  if (facts.onlyFished) lines.push(t("day.onlyFished"));

  if (facts.nothingHappened || lines.length === 0) {
    return <Text className="reading">{t("day.nothing")}</Text>;
  }
  return (
    <List className="reading" spacing={4}>
      {lines.map((line) => (
        <List.Item key={line}>{line}</List.Item>
      ))}
    </List>
  );
}
