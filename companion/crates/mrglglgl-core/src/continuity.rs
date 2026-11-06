//! The continuity sheet: what a character's story carries from one entry to the next.
//!
//! Facts are computed from the archive and never taken from a model: titles (milestones earned),
//! the nemesis (the creature that killed the character most), recurring companions, and the
//! running counts. The model may only propose running jokes, which are validated and capped. The
//! boot count in particular is always the journal's own number.

use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::beats::DayRecord;
use crate::db::{Database, DbError};
use crate::milestones::{self, Award};
use crate::schema::Journal;

/// Running jokes kept at most; the newest win.
pub const MAX_JOKES: usize = 5;
/// The longest running joke accepted, in characters.
pub const MAX_JOKE_CHARS: usize = 120;
/// Companions listed at most.
const MAX_COMPANIONS: usize = 5;
/// Deaths to one kind of creature before it counts as a nemesis.
const NEMESIS_DEATHS: i64 = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", default)]
pub struct Nemesis {
    pub name: String,
    pub deaths: i64,
}

/// A companion met on more than one day.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", default)]
pub struct Recurring {
    pub name: String,
    pub days: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", default)]
pub struct ContinuitySheet {
    /// Names of the milestones earned.
    pub titles: Vec<String>,
    pub nemesis: Option<Nemesis>,
    /// Companions met on more than one day, most often met first.
    pub companions: Vec<Recurring>,
    pub boots: i64,
    pub murlocs_slain: i64,
    pub deaths_by_murloc: i64,
    /// Proposed by the model, validated and capped.
    pub running_jokes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("the model's continuity update is not valid JSON: {0}")]
    Malformed(#[from] serde_json::Error),
}

/// What a model may change: running jokes, and nothing else.
#[derive(Deserialize)]
struct ModelUpdate {
    #[serde(alias = "running_jokes", rename = "runningJokes")]
    running_jokes: Vec<String>,
}

impl ContinuitySheet {
    /// The sheet as computed from the journal. With `before`, only days before it count and the
    /// running totals are what they were at the start of that day, so an old day written late is
    /// remembered as it was; without it, the totals are the journal's own.
    pub fn compute(
        journal: &Journal,
        awards: &[Award],
        running_jokes: &[String],
        before: Option<&str>,
    ) -> Self {
        let counted: Vec<(&String, DayRecord)> = journal
            .days
            .iter()
            .filter(|(d, _)| before.is_none_or(|b| d.as_str() < b))
            .map(|(d, day)| (d, DayRecord::of(day)))
            .collect();

        let mut killers: BTreeMap<&str, i64> = BTreeMap::new();
        let mut met: BTreeMap<&str, i64> = BTreeMap::new();
        for (_, r) in &counted {
            for d in &r.deaths {
                if let Some(k) = &d.killer {
                    *killers.entry(k).or_default() += 1;
                }
            }
            for c in &r.companions {
                *met.entry(c).or_default() += 1;
            }
        }
        // Most deaths wins; ties go to the name first in alphabetical order, so the choice is
        // stable.
        let nemesis = killers
            .iter()
            .filter(|(_, n)| **n >= NEMESIS_DEATHS)
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(name, deaths)| Nemesis {
                name: name.to_string(),
                deaths: *deaths,
            });
        let mut companions: Vec<(&str, i64)> =
            met.into_iter().filter(|(_, days)| *days >= 2).collect();
        companions.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

        // Totals at the start of `before`: the journal's totals minus what that day and later
        // days added. The journal's totals can predate the archive, so they are the anchor.
        let later = |f: fn(&DayRecord) -> i64| -> i64 {
            before.map_or(0, |b| {
                journal
                    .days
                    .range(b.to_string()..)
                    .map(|(_, day)| f(&DayRecord::of(day)))
                    .sum()
            })
        };
        let stats = &journal.stats;
        Self {
            titles: awards
                .iter()
                .filter(|a| before.is_none_or(|b| a.day.as_str() < b))
                .map(|a| milestones::name(&a.id).to_string())
                .collect(),
            nemesis,
            companions: companions
                .into_iter()
                .take(MAX_COMPANIONS)
                .map(|(name, days)| Recurring {
                    name: name.to_string(),
                    days,
                })
                .collect(),
            boots: (stats.boots - later(|r| r.boots)).max(0),
            murlocs_slain: (stats.murlocs_slain - later(|r| r.murloc_kills())).max(0),
            deaths_by_murloc: (stats.deaths_by_murloc
                - later(|r| r.deaths.iter().filter(|d| d.murloc).count() as i64))
            .max(0),
            running_jokes: running_jokes.to_vec(),
        }
    }

    /// The sheet with a model's proposed running jokes added. Anything else in the proposal is
    /// ignored; jokes that are empty, too long or repeated are dropped, and only the newest
    /// [`MAX_JOKES`] are kept.
    pub fn with_model_update(&self, json: &str) -> Result<Self, UpdateError> {
        let update: ModelUpdate = serde_json::from_str(json)?;
        let mut jokes = self.running_jokes.clone();
        for joke in update.running_jokes {
            let joke = joke.trim().to_string();
            if joke.is_empty()
                || joke.chars().count() > MAX_JOKE_CHARS
                || jokes.iter().any(|j| j.eq_ignore_ascii_case(&joke))
            {
                continue;
            }
            jokes.push(joke);
        }
        let skip = jokes.len().saturating_sub(MAX_JOKES);
        Ok(Self {
            running_jokes: jokes.into_iter().skip(skip).collect(),
            ..self.clone()
        })
    }

    /// The sheet as lines of memory for the prompt, in English, worded as in the tested prompt
    /// with the nemesis and titles after. Empty facts are left out.
    pub fn memory_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.boots > 0 {
            lines.push(format!("Boots kept so far: {}", self.boots));
        }
        if self.murlocs_slain > 0 {
            lines.push(format!("Murlocs slain in total: {}", self.murlocs_slain));
        }
        if self.deaths_by_murloc > 0 {
            lines.push(format!(
                "Deaths by murloc in total: {}",
                self.deaths_by_murloc
            ));
        }
        for c in &self.companions {
            lines.push(format!(
                "Recurring companion: {} ({} days together)",
                c.name, c.days
            ));
        }
        for joke in &self.running_jokes {
            lines.push(format!("Running joke: {joke}"));
        }
        if let Some(n) = &self.nemesis {
            lines.push(format!(
                "Nemesis: {} (died to them {} times)",
                n.name, n.deaths
            ));
        }
        if !self.titles.is_empty() {
            lines.push(format!("Milestones earned: {}", self.titles.join(", ")));
        }
        lines
    }
}

/// The stored sheet, if the character has one.
pub fn load(db: &Database, character_id: i64) -> Result<Option<ContinuitySheet>, DbError> {
    let json: Option<String> = db
        .conn()
        .query_row(
            "SELECT sheet_json FROM continuity WHERE character_id = ?1",
            [character_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(json.and_then(|j| match serde_json::from_str(&j) {
        Ok(sheet) => Some(sheet),
        Err(e) => {
            tracing::warn!("continuity sheet of character {character_id} is unreadable: {e}");
            None
        }
    }))
}

fn save(db: &Database, character_id: i64, sheet: &ContinuitySheet) -> Result<(), DbError> {
    let json = serde_json::to_string(sheet).expect("a sheet always serialises");
    db.conn().execute(
        "INSERT INTO continuity (character_id, sheet_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (character_id) DO UPDATE SET
             sheet_json = excluded.sheet_json, updated_at = excluded.updated_at",
        params![character_id, json, chrono::Utc::now().timestamp()],
    )?;
    Ok(())
}

/// Recomputes and stores a character's sheet after an entry was written. `model_update` is the
/// model's proposal, if it made one; a malformed proposal is logged and leaves the jokes as they
/// were. The facts always come from the journal.
pub fn refresh(
    db: &Database,
    character_id: i64,
    journal: &Journal,
    awards: &[Award],
    model_update: Option<&str>,
) -> Result<ContinuitySheet, DbError> {
    let jokes = load(db, character_id)?
        .map(|s| s.running_jokes)
        .unwrap_or_default();
    let mut sheet = ContinuitySheet::compute(journal, awards, &jokes, None);
    if let Some(json) = model_update {
        match sheet.with_model_update(json) {
            Ok(updated) => sheet = updated,
            Err(e) => tracing::warn!("character {character_id}: {e}; sheet left unchanged"),
        }
    }
    save(db, character_id, &sheet)?;
    Ok(sheet)
}
