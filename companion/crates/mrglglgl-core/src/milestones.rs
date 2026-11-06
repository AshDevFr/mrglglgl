//! Cosmetic milestones, awarded from the archive and shown as stamps in the journal.
//!
//! Each milestone is data: an id, whether it is still a proposal, whether it is on, and a rule
//! over a day's facts. Facts come from a day's raw events when the archive has them and from its
//! digest otherwise, so days the addon compacted count too. Awards are stored once and never move:
//! re-importing, or learning of an earlier qualifying day later, does not change a stamp already
//! given.

use std::collections::BTreeMap;

use rusqlite::params;
use serde::Serialize;

use crate::db::{Database, DbError};
use crate::schema::{Day, EventKind, Journal};

/// What happened on one day, as far as milestones care.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayFacts {
    pub boots: i64,
    pub fish: i64,
    pub murloc_kills: i64,
    pub deaths: i64,
    pub deaths_by_murloc: i64,
    pub drownings: i64,
    pub discoveries: i64,
    pub only_fished: bool,
    pub nothing_happened: bool,
}

enum Rule {
    /// The first day on which the predicate holds.
    FirstDay(fn(&DayFacts) -> bool),
    /// The day a running total, counted day by day, reaches the threshold.
    Total(fn(&DayFacts) -> i64, i64),
}

pub struct Milestone {
    pub id: &'static str,
    /// Suggested but not yet approved by the project owner.
    pub proposed: bool,
    /// Turning a milestone off is this one flag.
    pub enabled: bool,
    rule: Rule,
}

/// Every milestone, in the order the journal lists them. Names and descriptions live in the UI's
/// locale catalogues under the id.
pub const MILESTONES: &[Milestone] = &[
    // The first stamp most players get: a single boot is enough.
    Milestone {
        id: "kept_the_boot",
        proposed: false,
        enabled: true,
        rule: Rule::FirstDay(|f| f.boots > 0),
    },
    Milestone {
        id: "first_murloc",
        proposed: false,
        enabled: true,
        rule: Rule::FirstDay(|f| f.murloc_kills > 0),
    },
    Milestone {
        id: "slain_by_murloc",
        proposed: false,
        enabled: true,
        rule: Rule::FirstDay(|f| f.deaths_by_murloc > 0),
    },
    Milestone {
        id: "traitor",
        proposed: false,
        enabled: true,
        rule: Rule::Total(|f| f.murloc_kills, 100),
    },
    Milestone {
        id: "only_fished",
        proposed: false,
        enabled: true,
        rule: Rule::FirstDay(|f| f.only_fished),
    },
    Milestone {
        id: "boot_collector",
        proposed: true,
        enabled: true,
        rule: Rule::Total(|f| f.boots, 5),
    },
    Milestone {
        id: "sleeps_with_fishes",
        proposed: true,
        enabled: true,
        rule: Rule::FirstDay(|f| f.drownings > 0),
    },
    Milestone {
        id: "bad_day",
        proposed: true,
        enabled: true,
        rule: Rule::FirstDay(|f| f.deaths >= 3),
    },
    Milestone {
        id: "tourist",
        proposed: true,
        enabled: true,
        rule: Rule::FirstDay(|f| f.discoveries >= 5),
    },
    Milestone {
        id: "quiet_day",
        proposed: true,
        enabled: true,
        rule: Rule::FirstDay(|f| f.nothing_happened),
    },
];

/// The facts of one day, from raw events when present, from the digest otherwise.
pub fn facts(day: &Day) -> DayFacts {
    let digest = day.digest.as_ref();
    let Some(events) = &day.events else {
        let Some(d) = digest else {
            return DayFacts::default();
        };
        return DayFacts {
            boots: d.counts.boots,
            fish: d.counts.fish,
            murloc_kills: d.counts.murlocs,
            deaths: d.counts.deaths,
            deaths_by_murloc: d
                .deaths
                .iter()
                .filter(|death| death.killer.as_ref().is_some_and(|k| k.murloc))
                .count() as i64,
            drownings: d
                .deaths
                .iter()
                .filter(|death| death.environment.as_deref() == Some("Drowning"))
                .count() as i64,
            discoveries: d.counts.discoveries,
            only_fished: d.only_fished,
            nothing_happened: d.nothing_happened,
        };
    };

    let mut f = DayFacts {
        murloc_kills: day
            .kills
            .values()
            .filter(|k| k.murloc)
            .map(|k| k.count)
            .sum(),
        ..DayFacts::default()
    };
    let kills: i64 = day.kills.values().map(|k| k.count).sum::<i64>() + day.anon_kills;
    let mut active = kills > 0;
    let mut quests = 0;
    for event in events {
        match &event.kind {
            EventKind::Fish { boot, .. } => {
                f.fish += 1;
                if *boot == Some(true) {
                    f.boots += 1;
                }
            }
            EventKind::Death {
                killer,
                environment,
            } => {
                f.deaths += 1;
                if killer.as_ref().is_some_and(|k| k.murloc) {
                    f.deaths_by_murloc += 1;
                }
                if environment.as_deref() == Some("Drowning") {
                    f.drownings += 1;
                }
                active = true;
            }
            EventKind::Discover { .. } => {
                f.discoveries += 1;
                active = true;
            }
            EventKind::QuestAccept { .. } | EventKind::QuestTurnin { .. } => {
                quests += 1;
                active = true;
            }
            EventKind::Level
            | EventKind::Loot { .. }
            | EventKind::Boss { .. }
            | EventKind::GroupJoin { .. } => active = true,
            _ => {}
        }
    }
    // The digest's definitions: fishing with no kills or quests; and nothing worth a line.
    f.only_fished = digest.map_or(f.fish > 0 && kills == 0 && quests == 0, |d| d.only_fished);
    f.nothing_happened = digest.map_or(!active && f.fish == 0, |d| d.nothing_happened);
    f
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Award {
    pub id: String,
    /// The day it was earned, `YYYY-MM-DD`.
    pub day: String,
}

/// The milestones a journal has earned and the first day each was earned, from its days alone.
pub fn compute(journal: &Journal) -> Vec<Award> {
    let days: Vec<(&String, DayFacts)> = journal
        .days
        .iter()
        .map(|(date, day)| (date, facts(day)))
        .collect();
    MILESTONES
        .iter()
        .filter(|m| m.enabled)
        .filter_map(|m| {
            let day = match m.rule {
                Rule::FirstDay(holds) => days.iter().find(|(_, f)| holds(f)).map(|(d, _)| *d),
                Rule::Total(count, threshold) => {
                    let mut total = 0;
                    days.iter()
                        .find(|(_, f)| {
                            total += count(f);
                            total >= threshold
                        })
                        .map(|(d, _)| *d)
                }
            }?;
            Some(Award {
                id: m.id.to_string(),
                day: day.clone(),
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
    Ok(MILESTONES
        .iter()
        .filter(|m| m.enabled)
        .filter_map(|m| {
            stored.get(m.id).map(|day| Award {
                id: m.id.to_string(),
                day: day.clone(),
            })
        })
        .collect())
}
