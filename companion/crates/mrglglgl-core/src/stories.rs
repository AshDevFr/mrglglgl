//! Written entries. Every version is kept; the newest is the one shown, older ones are one
//! click away.

use std::collections::BTreeMap;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::{Database, DbError};
use crate::writer::CheckStatus;

/// What was written: one day, or a week as a chapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum StoryKind {
    Day,
    Week,
}

impl StoryKind {
    pub fn key(self) -> &'static str {
        match self {
            StoryKind::Day => "day",
            StoryKind::Week => "week",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "day" => Some(StoryKind::Day),
            "week" => Some(StoryKind::Week),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Story {
    pub id: i64,
    pub character_id: i64,
    pub kind: StoryKind,
    /// The day, or the week's Monday.
    pub start_day: String,
    /// The day, or the week's Sunday.
    pub end_day: String,
    pub narrator: String,
    pub language: String,
    pub provider: String,
    pub model: String,
    pub prompt_hash: String,
    pub text: String,
    /// Unix time.
    pub created_at: i64,
    pub check: CheckStatus,
    /// Why the entry was set aside, when the fact check failed; the day then shows its facts.
    pub check_reason: Option<String>,
}

/// A story about to be stored.
#[derive(Debug, Clone)]
pub struct NewStory {
    pub character_id: i64,
    pub kind: StoryKind,
    pub start_day: String,
    pub end_day: String,
    pub narrator: String,
    pub language: String,
    pub provider: String,
    pub model: String,
    pub prompt_hash: String,
    pub text: String,
    pub check: CheckStatus,
    pub check_reason: Option<String>,
}

/// Stores a new version and returns its id.
pub fn save(db: &Database, story: &NewStory) -> Result<i64, DbError> {
    Ok(db.conn().query_row(
        "INSERT INTO stories (character_id, kind, start_day, end_day, narrator, language,
             provider, model, prompt_hash, text, created_at, check_status, check_reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         RETURNING id",
        params![
            story.character_id,
            story.kind.key(),
            story.start_day,
            story.end_day,
            story.narrator,
            story.language,
            story.provider,
            story.model,
            story.prompt_hash,
            story.text,
            chrono::Utc::now().timestamp(),
            story.check.key(),
            story.check_reason,
        ],
        |row| row.get(0),
    )?)
}

const COLUMNS: &str = "id, character_id, kind, start_day, end_day, narrator, language, provider,
     model, prompt_hash, text, created_at, check_status, check_reason";

fn read(row: &rusqlite::Row) -> rusqlite::Result<Story> {
    Ok(Story {
        id: row.get(0)?,
        character_id: row.get(1)?,
        kind: StoryKind::from_key(&row.get::<_, String>(2)?).unwrap_or(StoryKind::Day),
        start_day: row.get(3)?,
        end_day: row.get(4)?,
        narrator: row.get(5)?,
        language: row.get(6)?,
        provider: row.get(7)?,
        model: row.get(8)?,
        prompt_hash: row.get(9)?,
        text: row.get(10)?,
        created_at: row.get(11)?,
        check: CheckStatus::from_key(&row.get::<_, String>(12)?).unwrap_or(CheckStatus::Passed),
        check_reason: row.get(13)?,
    })
}

/// Every version for one day or week, newest first.
pub fn versions(
    db: &Database,
    character_id: i64,
    kind: StoryKind,
    start_day: &str,
) -> Result<Vec<Story>, DbError> {
    let mut stmt = db.conn().prepare(&format!(
        "SELECT {COLUMNS} FROM stories
         WHERE character_id = ?1 AND kind = ?2 AND start_day = ?3
         ORDER BY id DESC"
    ))?;
    let rows = stmt.query_map(params![character_id, kind.key(), start_day], read)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every version of a character's stories, newest first within each day or week.
pub fn all(db: &Database, character_id: i64) -> Result<Vec<Story>, DbError> {
    let mut stmt = db.conn().prepare(&format!(
        "SELECT {COLUMNS} FROM stories WHERE character_id = ?1
         ORDER BY kind, start_day, id DESC"
    ))?;
    let rows = stmt.query_map([character_id], read)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The newest version of each day or week of `kind`, by start day.
pub fn latest(
    db: &Database,
    character_id: i64,
    kind: StoryKind,
) -> Result<BTreeMap<String, Story>, DbError> {
    let mut latest = BTreeMap::new();
    for story in all(db, character_id)?
        .into_iter()
        .filter(|s| s.kind == kind)
    {
        latest.entry(story.start_day.clone()).or_insert(story);
    }
    Ok(latest)
}

/// The text of the newest entry of each day that passed the fact check, for a weekly chapter.
pub fn day_entries(db: &Database, character_id: i64) -> Result<BTreeMap<String, String>, DbError> {
    Ok(latest(db, character_id, StoryKind::Day)?
        .into_iter()
        .filter(|(_, s)| s.check != CheckStatus::Failed)
        .map(|(day, s)| (day, s.text))
        .collect())
}
