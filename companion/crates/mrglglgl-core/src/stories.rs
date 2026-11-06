//! Written entries. Every version is kept; the newest accepted one is the one shown, older ones
//! are one click away. A draft waits for the player to accept or discard it, and only accepted
//! entries reach the journal, copies, exports and weekly chapters.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{OptionalExtension, params};
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

/// Where a story stands with the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum StoryStatus {
    /// In the journal.
    Accepted,
    /// Written, waiting for the player to accept or discard it.
    Draft,
    /// A draft the player threw away; kept so the day is not silently written again.
    Discarded,
}

impl StoryStatus {
    pub fn key(self) -> &'static str {
        match self {
            StoryStatus::Accepted => "accepted",
            StoryStatus::Draft => "draft",
            StoryStatus::Discarded => "discarded",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "draft" => StoryStatus::Draft,
            "discarded" => StoryStatus::Discarded,
            _ => StoryStatus::Accepted,
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
    pub status: StoryStatus,
    /// What the player asked of this version when they asked for a rewrite.
    pub note: Option<String>,
    /// Written while the day was still going: what happened so far.
    pub partial: bool,
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
    pub status: StoryStatus,
    pub note: Option<String>,
    pub partial: bool,
}

/// Stores a new version and returns its id.
pub fn save(db: &Database, story: &NewStory) -> Result<i64, DbError> {
    Ok(db.conn().query_row(
        "INSERT INTO stories (character_id, kind, start_day, end_day, narrator, language,
             provider, model, prompt_hash, text, created_at, check_status, check_reason, status,
             note, partial)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
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
            story.status.key(),
            story.note,
            story.partial,
        ],
        |row| row.get(0),
    )?)
}

const COLUMNS: &str = "id, character_id, kind, start_day, end_day, narrator, language, provider,
     model, prompt_hash, text, created_at, check_status, check_reason, status, note,
     partial";

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
        status: StoryStatus::from_key(&row.get::<_, String>(14)?),
        note: row.get(15)?,
        partial: row.get(16)?,
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

/// Every version of a character's stories, drafts and discarded ones included, newest first
/// within each day or week.
pub fn all(db: &Database, character_id: i64) -> Result<Vec<Story>, DbError> {
    let mut stmt = db.conn().prepare(&format!(
        "SELECT {COLUMNS} FROM stories WHERE character_id = ?1
         ORDER BY kind, start_day, id DESC"
    ))?;
    let rows = stmt.query_map([character_id], read)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every accepted version of a character's stories, newest first within each day or week: what
/// the journal, copies and exports show.
pub fn accepted(db: &Database, character_id: i64) -> Result<Vec<Story>, DbError> {
    Ok(all(db, character_id)?
        .into_iter()
        .filter(|s| s.status == StoryStatus::Accepted)
        .collect())
}

/// The newest accepted version of each day or week of `kind`, by start day.
pub fn latest(
    db: &Database,
    character_id: i64,
    kind: StoryKind,
) -> Result<BTreeMap<String, Story>, DbError> {
    let mut latest = BTreeMap::new();
    for story in accepted(db, character_id)?
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
        .filter(|(_, s)| s.check != CheckStatus::Failed && !s.partial)
        .map(|(day, s)| (day, s.text))
        .collect())
}

/// Every day and week with a story of any status: the background writer leaves these alone, so a
/// draft is not written twice and a discarded day is only written again when the player asks. A
/// day told only so far does not count: its full entry is still to be written.
pub fn slots(db: &Database, character_id: i64) -> Result<BTreeSet<(StoryKind, String)>, DbError> {
    Ok(all(db, character_id)?
        .into_iter()
        .filter(|s| !s.partial)
        .map(|s| (s.kind, s.start_day))
        .collect())
}

/// Accepts a draft into the journal and discards the other drafts for the same day or week.
/// Returns false when `story_id` is not a draft of this character.
pub fn accept(db: &Database, character_id: i64, story_id: i64) -> Result<bool, DbError> {
    let conn = db.conn();
    let tx = conn.unchecked_transaction()?;
    let accepted = tx.execute(
        "UPDATE stories SET status = 'accepted'
         WHERE id = ?1 AND character_id = ?2 AND status = 'draft'",
        params![story_id, character_id],
    )? > 0;
    if accepted {
        tx.execute(
            "UPDATE stories SET status = 'discarded'
             WHERE status = 'draft' AND id != ?1 AND (character_id, kind, start_day) =
                 (SELECT character_id, kind, start_day FROM stories WHERE id = ?1)",
            [story_id],
        )?;
    }
    tx.commit()?;
    Ok(accepted)
}

/// Who wrote a version the player edited by hand.
pub const PLAYER: &str = "player";

/// The longest entry the player may save, in characters.
pub const MAX_EDIT_CHARS: usize = 20_000;

/// Saves the player's own text for a day or week as a new version, in the journal at once, made
/// from `story_id` (whose narrator and language it keeps). Drafts of the same day or week are set
/// aside, as when a draft is accepted. Returns the new version's id, or `None` when `story_id` is
/// not this character's, is a day told only so far (its full entry would replace the edit), or
/// the text is blank.
pub fn edit(
    db: &Database,
    character_id: i64,
    story_id: i64,
    text: &str,
) -> Result<Option<i64>, DbError> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > MAX_EDIT_CHARS {
        return Ok(None);
    }
    let base = db
        .conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM stories WHERE id = ?1 AND character_id = ?2"),
            params![story_id, character_id],
            read,
        )
        .optional()?;
    let Some(base) = base.filter(|b| !b.partial) else {
        return Ok(None);
    };
    let id = save(
        db,
        &NewStory {
            character_id,
            kind: base.kind,
            start_day: base.start_day,
            end_day: base.end_day,
            narrator: base.narrator,
            language: base.language,
            provider: PLAYER.to_string(),
            model: String::new(),
            prompt_hash: base.prompt_hash,
            text: text.to_string(),
            check: CheckStatus::Edited,
            check_reason: None,
            status: StoryStatus::Accepted,
            note: None,
            partial: false,
        },
    )?;
    db.conn().execute(
        "UPDATE stories SET status = 'discarded'
         WHERE status = 'draft' AND (character_id, kind, start_day) =
             (SELECT character_id, kind, start_day FROM stories WHERE id = ?1)",
        [id],
    )?;
    Ok(Some(id))
}

/// Throws a draft away. Returns false when `story_id` is not a draft of this character: an
/// accepted entry is never discarded.
pub fn discard(db: &Database, character_id: i64, story_id: i64) -> Result<bool, DbError> {
    Ok(db.conn().execute(
        "UPDATE stories SET status = 'discarded'
         WHERE id = ?1 AND character_id = ?2 AND status = 'draft'",
        params![story_id, character_id],
    )? > 0)
}
