//! Cosmetic milestones, awarded from the archive and shown as stamps in the journal.
//!
//! The milestones themselves are data, in `data/milestones.toml`: an id, a rule over a day's facts
//! and a threshold. Facts come from a day's raw events when the archive has them and from its
//! digest otherwise, so days the addon compacted count too. Awards are stored once and never move:
//! re-importing, or learning of an earlier qualifying day later, does not change a stamp already
//! given.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use chrono::{Local, TimeZone, Timelike};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::{Database, DbError};
use crate::schema::{Day, EventKind, Journal};

/// What happened on one day, as far as milestones care. Every field can be named as a `fact` in
/// `data/milestones.toml`; true counts as 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayFacts {
    pub boots: i64,
    pub fish: i64,
    pub junk: i64,
    pub kills: i64,
    pub murloc_kills: i64,
    /// Elites killed, rare elites included.
    pub elites: i64,
    /// Rares killed, rare elites included.
    pub rares: i64,
    pub bosses: i64,
    pub deaths: i64,
    pub deaths_by_murloc: i64,
    pub drownings: i64,
    pub falls: i64,
    pub lava: i64,
    pub discoveries: i64,
    pub levels_gained: i64,
    /// The highest level seen that day.
    pub level: i64,
    pub quests_done: i64,
    /// Dungeon and raid quests handed in; needs the journal's quest records.
    pub dungeon_quests_done: i64,
    /// Copper earned and spent.
    pub money_in: i64,
    pub money_out: i64,
    pub auctions_sold: i64,
    pub crafted: i64,
    /// The highest rank a profession reached that day.
    pub profession_rank: i64,
    pub group_joins: i64,
    /// The player's own lines said aloud (say, emote, yell).
    pub own_lines: i64,
    /// Legacy Challenges (achievements) completed.
    pub challenges: i64,
    /// Sessions under way between 3 and 6 in the morning, local time.
    pub night_sessions: i64,
    pub only_fished: bool,
    pub nothing_happened: bool,
}

impl DayFacts {
    /// The fact named in the milestones file; none for a name that is not a fact.
    pub fn value(&self, fact: &str) -> Option<i64> {
        Some(match fact {
            "boots" => self.boots,
            "fish" => self.fish,
            "junk" => self.junk,
            "kills" => self.kills,
            "murloc_kills" => self.murloc_kills,
            "elites" => self.elites,
            "rares" => self.rares,
            "bosses" => self.bosses,
            "deaths" => self.deaths,
            "deaths_by_murloc" => self.deaths_by_murloc,
            "drownings" => self.drownings,
            "falls" => self.falls,
            "lava" => self.lava,
            "discoveries" => self.discoveries,
            "levels_gained" => self.levels_gained,
            "level" => self.level,
            "quests_done" => self.quests_done,
            "dungeon_quests_done" => self.dungeon_quests_done,
            "money_in" => self.money_in,
            "money_out" => self.money_out,
            "auctions_sold" => self.auctions_sold,
            "crafted" => self.crafted,
            "profession_rank" => self.profession_rank,
            "group_joins" => self.group_joins,
            "own_lines" => self.own_lines,
            "challenges" => self.challenges,
            "night_sessions" => self.night_sessions,
            "only_fished" => self.only_fished as i64,
            "nothing_happened" => self.nothing_happened as i64,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// The first day on which the fact reaches the threshold.
    FirstDay,
    /// The day the fact, added up day by day, reaches the threshold.
    Total,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Milestone {
    pub id: String,
    /// The English name, as prompts and exports use it. The UI translates by id instead.
    pub name: String,
    /// The French name, for French prose.
    #[serde(default)]
    pub name_fr: Option<String>,
    /// 1 (most players earn it early) to 5 (few ever will).
    pub rarity: i64,
    /// Suggested but not yet approved by the project owner.
    #[serde(default)]
    pub proposed: bool,
    /// Turning a milestone off is this one flag.
    #[serde(default = "on")]
    pub enabled: bool,
    pub rule: Rule,
    pub fact: String,
    pub at_least: i64,
}

fn on() -> bool {
    true
}

#[derive(Deserialize)]
struct MilestoneFile {
    milestone: Vec<Milestone>,
}

/// Every milestone, in the order the journal lists them.
pub fn all() -> &'static [Milestone] {
    static ALL: OnceLock<Vec<Milestone>> = OnceLock::new();
    ALL.get_or_init(|| {
        toml::from_str::<MilestoneFile>(include_str!("../data/milestones.toml"))
            .expect("data/milestones.toml matches Milestone")
            .milestone
    })
}

/// How many stamps of one day a narrator hears of, and the day's page presses: the rarest.
pub const TOLD_PER_DAY: usize = 3;

/// The rarest `n` of `awards`, rarest first; among equals, the journal's list order.
pub fn rarest<'a>(awards: impl IntoIterator<Item = &'a Award>, n: usize) -> Vec<&'a Award> {
    let order = |id: &str| all().iter().position(|m| m.id == id).unwrap_or(usize::MAX);
    let mut list: Vec<&Award> = awards.into_iter().collect();
    list.sort_by_key(|a| (std::cmp::Reverse(a.rarity), order(&a.id)));
    list.truncate(n);
    list
}

/// A milestone's name in the prose language: French for `"fr"` when it has one, English otherwise.
pub fn name_in<'a>(id: &'a str, language: &str) -> &'a str {
    let found = all().iter().find(|m| m.id == id);
    match found {
        Some(m) if language.starts_with("fr") => m.name_fr.as_deref().unwrap_or(&m.name),
        Some(m) => &m.name,
        None => id,
    }
}

/// The English name of a milestone, or its id when unknown.
pub fn name(id: &str) -> &str {
    all()
        .iter()
        .find(|m| m.id == id)
        .map_or(id, |m| m.name.as_str())
}

fn is_profession(id: Option<i64>, category: Option<&str>) -> bool {
    crate::beats::is_profession(id, category)
}

/// Local hours, 3 to 6 in the morning, that make a session a late one.
fn at_night(time: i64) -> bool {
    Local
        .timestamp_opt(time, 0)
        .single()
        .is_some_and(|t| (3..6).contains(&t.hour()))
}

/// The facts of one day, from raw events when present, from the digest otherwise. Facts that
/// need the journal's quest records (dungeon quests) are left at 0; [`compute`] fills them.
pub fn facts(day: &Day) -> DayFacts {
    let digest = day.digest.as_ref();
    let mut f = DayFacts::default();

    // What every day keeps whatever its state: money, chat (the archive keeps it), crafting.
    let money = if day.money.is_empty() {
        digest.map(|d| d.money.clone()).unwrap_or_default()
    } else {
        day.money.clone()
    };
    f.money_in = money.earned();
    f.money_out = money.spent();
    f.own_lines = day
        .chat
        .iter()
        .filter(|l| l.own() && matches!(l.channel.as_str(), "say" | "emote" | "yell"))
        .count() as i64;
    f.crafted = if day.crafted.is_empty() {
        digest.map_or(0, |d| d.counts.crafted)
    } else {
        day.crafted.values().map(|i| i.count).sum()
    };
    f.profession_rank = if day.skills.is_empty() {
        digest.map_or(0, |d| {
            d.skills
                .iter()
                .filter(|s| is_profession(s.skill_id, s.category.as_deref()))
                .map(|s| s.to)
                .max()
                .unwrap_or(0)
        })
    } else {
        day.skills
            .iter()
            .filter(|(id, s)| is_profession(Some(**id), s.category.as_deref()))
            .map(|(_, s)| s.to)
            .max()
            .unwrap_or(0)
    };
    // Kept on a compacted day too.
    for event in day.events.iter().flatten() {
        match &event.kind {
            EventKind::Achievement { .. } => f.challenges += 1,
            EventKind::Auction { act, .. } if act.as_deref() == Some("sold") => {
                f.auctions_sold += 1
            }
            _ => {}
        }
        if at_night(event.time) && matches!(event.kind, EventKind::SessionStart) {
            f.night_sessions += 1;
        }
        f.level = f.level.max(event.level.unwrap_or(0));
    }

    let Some(events) = day.events.as_ref().filter(|_| !day.compacted) else {
        let Some(d) = digest else {
            return f;
        };
        let notable = || d.kills.iter().chain(d.notable_kills.iter());
        let count_class = |classes: &[&str]| {
            let mut seen = std::collections::BTreeSet::new();
            notable()
                .filter(|k| classes.contains(&k.classification.as_str()))
                .filter(|k| seen.insert(k.npc_id))
                .map(|k| k.count)
                .sum::<i64>()
        };
        let env = |name: &str| {
            d.deaths
                .iter()
                .filter(|death| death.environment.as_deref() == Some(name))
                .count() as i64
        };
        if day.events.is_none() {
            f.challenges = d.counts.achievements;
            f.auctions_sold = d
                .auctions
                .iter()
                .filter(|a| a.act.as_deref() == Some("sold"))
                .count() as i64;
        }
        return DayFacts {
            boots: d.counts.boots,
            fish: d.counts.fish,
            junk: d.counts.junk,
            kills: d.counts.kills,
            murloc_kills: d.counts.murlocs,
            elites: count_class(&["elite", "rareelite"]),
            rares: count_class(&["rare", "rareelite"]),
            bosses: d.counts.bosses,
            deaths: d.counts.deaths,
            deaths_by_murloc: d
                .deaths
                .iter()
                .filter(|death| death.killer.as_ref().is_some_and(|k| k.murloc))
                .count() as i64,
            drownings: env("Drowning"),
            falls: env("Falling"),
            lava: env("Lava"),
            discoveries: d.counts.discoveries,
            levels_gained: d.counts.levels,
            level: f.level.max(d.level_to.unwrap_or(0)),
            quests_done: d.counts.turned_in,
            only_fished: d.only_fished,
            nothing_happened: d.nothing_happened,
            ..f
        };
    };

    f.murloc_kills = day
        .kills
        .values()
        .filter(|k| k.murloc)
        .map(|k| k.count)
        .sum();
    let class = |classes: &[&str]| {
        day.kills
            .values()
            .filter(|k| classes.contains(&k.classification.as_str()))
            .map(|k| k.count)
            .sum::<i64>()
    };
    f.elites = class(&["elite", "rareelite"]);
    f.rares = class(&["rare", "rareelite"]);
    let kills: i64 = day.kills.values().map(|k| k.count).sum::<i64>() + day.anon_kills;
    f.kills = kills;
    let mut active = kills > 0;
    let mut quests = 0;
    for event in events {
        match &event.kind {
            EventKind::Fish { boot, junk, .. } => {
                f.fish += 1;
                if *boot == Some(true) {
                    f.boots += 1;
                }
                if *junk == Some(true) {
                    f.junk += 1;
                }
            }
            EventKind::Death {
                killer,
                environment,
                ..
            } => {
                f.deaths += 1;
                if killer.as_ref().is_some_and(|k| k.murloc) {
                    f.deaths_by_murloc += 1;
                }
                match environment.as_deref() {
                    Some("Drowning") => f.drownings += 1,
                    Some("Falling") => f.falls += 1,
                    Some("Lava") => f.lava += 1,
                    _ => {}
                }
                active = true;
            }
            EventKind::Discover { .. } => {
                f.discoveries += 1;
                active = true;
            }
            EventKind::QuestAccept { .. } => {
                quests += 1;
                active = true;
            }
            EventKind::QuestTurnin { .. } => {
                quests += 1;
                f.quests_done += 1;
                active = true;
            }
            EventKind::Level => {
                f.levels_gained += 1;
                active = true;
            }
            EventKind::Boss { .. } => {
                f.bosses += 1;
                active = true;
            }
            EventKind::GroupJoin { .. } => {
                f.group_joins += 1;
                active = true;
            }
            EventKind::Loot { .. } => active = true,
            _ => {}
        }
    }
    // The digest's definitions: fishing with no kills or quests; and nothing worth a line.
    f.only_fished = digest.map_or(f.fish > 0 && kills == 0 && quests == 0, |d| d.only_fished);
    f.nothing_happened = digest.map_or(!active && f.fish == 0, |d| d.nothing_happened);
    f
}

/// Dungeon and raid quests handed in on `day`, from the journal's quest records.
fn dungeon_quests(journal: &Journal, day: &Day) -> i64 {
    let turned_in: Vec<i64> = match day.events.as_ref().filter(|_| !day.compacted) {
        Some(events) => events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::QuestTurnin { quest_id, .. } => quest_id,
                _ => None,
            })
            .collect(),
        None => day
            .digest
            .as_ref()
            .map(|d| d.turned_in.clone())
            .unwrap_or_default(),
    };
    turned_in
        .iter()
        .filter(|id| {
            journal
                .quests
                .get(id)
                .and_then(|q| q.kind.as_deref())
                .is_some_and(|k| matches!(k, "dungeon" | "raid"))
        })
        .count() as i64
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Award {
    pub id: String,
    /// The day it was earned, `YYYY-MM-DD`.
    pub day: String,
    /// The milestone's rarity, 1 to 5.
    pub rarity: i64,
}

/// The milestones a journal has earned and the first day each was earned, from its days alone.
pub fn compute(journal: &Journal) -> Vec<Award> {
    let days: Vec<(&String, DayFacts)> = journal
        .days
        .iter()
        .map(|(date, day)| {
            let mut f = facts(day);
            f.dungeon_quests_done = dungeon_quests(journal, day);
            (date, f)
        })
        .collect();
    all()
        .iter()
        .filter(|m| m.enabled)
        .filter_map(|m| {
            let value = |f: &DayFacts| f.value(&m.fact).unwrap_or(0);
            let day = match m.rule {
                Rule::FirstDay => days
                    .iter()
                    .find(|(_, f)| value(f) >= m.at_least)
                    .map(|(d, _)| *d),
                Rule::Total => {
                    let mut total = 0;
                    days.iter()
                        .find(|(_, f)| {
                            total += value(f);
                            total >= m.at_least
                        })
                        .map(|(d, _)| *d)
                }
            }?;
            Some(Award {
                id: m.id.to_string(),
                day: day.clone(),
                rarity: m.rarity,
            })
        })
        .collect()
}

/// Stores the milestones a character has newly earned and returns all it holds, in list order.
/// A stamp already given keeps its day.
pub fn award(db: &Database, character_id: i64, journal: &Journal) -> Result<Vec<Award>, DbError> {
    let conn = db.conn();
    for award in compute(journal) {
        conn.execute(
            "INSERT OR IGNORE INTO milestones (character_id, id, day) VALUES (?1, ?2, ?3)",
            params![character_id, award.id, award.day],
        )?;
    }
    awards(db, character_id)
}

/// The milestones a character holds, in list order. Ones since turned off are left out.
pub fn awards(db: &Database, character_id: i64) -> Result<Vec<Award>, DbError> {
    let mut stmt = db
        .conn()
        .prepare("SELECT id, day FROM milestones WHERE character_id = ?1")?;
    let stored: BTreeMap<String, String> = stmt
        .query_map([character_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(all()
        .iter()
        .filter(|m| m.enabled)
        .filter_map(|m| {
            stored.get(&m.id).map(|day| Award {
                id: m.id.to_string(),
                day: day.clone(),
                rarity: m.rarity,
            })
        })
        .collect())
}
