//! One-line summaries of quests ("Apothecary Helbrim wanted a sample of wolf hearts"), written by
//! the model from the quest's own texts and kept, so an entry can tell what an errand was about
//! without carrying its description. A fast-levelling day touches dozens of quests whose texts
//! alone would fill a small model's context.
//!
//! A day's quests are summed up just before its entry is written; in idle time the rest are, the
//! newest first. Each gist is checked against the texts it came from and kept only if it names
//! nothing they do not. A gist is kept per quest and prose language, with a hash of the texts it
//! came from: a hand-in text that arrives later asks for a new one, and a gist that failed its
//! check is not asked for again until the texts change.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use mrglglgl_llm::{
    CancellationToken, DEFAULT_TEMPERATURE, GenerateOutcome, GenerateRequest, Provider,
    generate_text,
};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};

use crate::archive;
use crate::beats::DayRecord;
use crate::db::{Database, DbError};
use crate::factcheck::{self, FactSet};
use crate::prompt;
use crate::queue::Control;
use crate::schema::{Journal, Quest};
use crate::usage::Audience;

/// The longest gist asked for, in words.
pub const GIST_WORDS: usize = 20;
/// A gist longer than this is not a gist: the model ignored the request.
const GIST_WORDS_KEPT: usize = GIST_WORDS * 2;
/// Room for the line, and for a reasoning model's thinking.
const GIST_TOKENS: u32 = 400;
/// A gist is short; a model that takes longer is stuck.
const GIST_TIMEOUT: Duration = Duration::from_secs(90);
/// Each part of a quest's text sent, in characters.
const TEXT_CHARS: usize = 1_000;

/// The texts a gist is written from, or `None` when the quest has nothing to sum up.
fn source(quest: &Quest) -> Option<String> {
    let title = quest.title.as_deref()?;
    let cut = |t: &str| t.trim().chars().take(TEXT_CHARS).collect::<String>();
    let text = quest.text.as_deref().map(cut).filter(|t| !t.is_empty());
    let done = quest.done.as_deref().map(cut).filter(|t| !t.is_empty());
    if text.is_none() && done.is_none() {
        return None;
    }
    let mut out = format!("Quest: \"{title}\"");
    if let Some(text) = text {
        out.push_str(&format!("\nWhat the quest giver asked: {text}"));
    }
    if let Some(done) = done {
        out.push_str(&format!("\nWhat they said when it was handed in: {done}"));
    }
    Some(out)
}

fn hash(source: &str) -> String {
    Sha256::digest(source.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn system(language: &str) -> String {
    format!(
        "You write one line about a World of Warcraft quest for an adventurer's journal: who \
         asked, what for, and why, from the quest's own words. At most {GIST_WORDS} words, in \
         {language}, as a plain statement in the past tense (\"a worried farmer wanted the \
         wolves near his fields culled\"). Name only people, places and things the quest names, \
         spelled exactly as given and never translated. No numbers the quest does not give. \
         Output only the line.",
        language = prompt::language_name(language),
    )
}

/// Every gist kept in `language`, by quest; the newest when a quest has several.
pub fn load(db: &Database, language: &str) -> Result<BTreeMap<i64, String>, DbError> {
    let mut stmt = db.conn().prepare(
        "SELECT quest_id, gist FROM quest_gists WHERE language = ?1 AND gist IS NOT NULL
         ORDER BY created_at",
    )?;
    let rows = stmt.query_map([language], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The gists of `journal`'s quests in `language`, each written from the texts the quest has now.
pub fn for_journal(
    db: &Database,
    journal: &Journal,
    language: &str,
) -> Result<BTreeMap<i64, String>, DbError> {
    let mut gists = BTreeMap::new();
    for (id, quest) in &journal.quests {
        let Some(source) = source(quest) else {
            continue;
        };
        if let Some(Some(gist)) = stored(db, *id, language, &source)? {
            gists.insert(*id, gist);
        }
    }
    Ok(gists)
}

/// The row for these texts: `Some(None)` when a gist was asked for and set aside.
fn stored(
    db: &Database,
    id: i64,
    language: &str,
    source: &str,
) -> Result<Option<Option<String>>, DbError> {
    Ok(db
        .conn()
        .query_row(
            "SELECT gist FROM quest_gists WHERE quest_id = ?1 AND language = ?2 AND source = ?3",
            params![id, language, hash(source)],
            |row| row.get(0),
        )
        .optional()?)
}

/// Whether `quest` has something to sum up that no gist (or failed attempt) covers yet.
fn wanted(db: &Database, id: i64, quest: &Quest, language: &str) -> Result<bool, DbError> {
    let Some(source) = source(quest) else {
        return Ok(false);
    };
    Ok(stored(db, id, language, &source)?.is_none())
}

fn store(
    db: &Database,
    id: i64,
    language: &str,
    source: &str,
    gist: Option<&str>,
) -> Result<(), DbError> {
    db.conn().execute(
        "INSERT OR REPLACE INTO quest_gists (quest_id, language, source, gist, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            language,
            hash(source),
            gist,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

/// The model's answer as one line, or `None` when it is not one.
fn one_line(answer: &str) -> Option<String> {
    let cleaned = crate::writer::clean(answer);
    let line = cleaned
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())?
        .trim_start_matches(['-', '*', ' '])
        .trim_matches(['"', '“', '”', ' '])
        .to_string();
    let words = line.split_whitespace().count();
    (words > 0 && words <= GIST_WORDS_KEPT).then_some(line)
}

/// How asking for one gist ended.
enum Asked {
    /// Kept, or set aside for good: either way the quest is done with.
    Settled(bool),
    /// The model could not be reached or was stopped: try again another time.
    Stopped,
}

async fn ask(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    id: i64,
    quest: &Quest,
    language: &str,
) -> Result<Asked, DbError> {
    let Some(source) = source(quest) else {
        return Ok(Asked::Settled(false));
    };
    let request = GenerateRequest {
        system: system(language),
        user: source.clone(),
        max_tokens: GIST_TOKENS,
        temperature: DEFAULT_TEMPERATURE,
        language: language.to_string(),
    };
    let answer = {
        let _turn = control.take_turn().await;
        generate_text(
            provider,
            request,
            GIST_TIMEOUT,
            CancellationToken::new(),
            None,
        )
        .await
    };
    let text = match answer {
        Ok((GenerateOutcome::Completed, text)) => text,
        Ok(_) => return Ok(Asked::Stopped),
        Err(e) => {
            tracing::debug!("quest {id}: no gist, the model failed: {e}");
            return Ok(Asked::Stopped);
        }
    };
    let gist = one_line(&text).filter(|line| {
        let claims = factcheck::check(line, &FactSet::from_notes(&source, &[]));
        if !claims.is_empty() {
            tracing::debug!(
                "quest {id}: gist set aside, it names {}",
                factcheck::reason(&claims)
            );
        }
        claims.is_empty()
    });
    store(&db.lock().unwrap(), id, language, &source, gist.as_deref())?;
    Ok(Asked::Settled(gist.is_some()))
}

/// What a run of gists came to.
#[derive(Default)]
struct Run {
    /// Quests done with: kept, or set aside for good.
    settled: usize,
    kept: usize,
    /// The model stopped answering; nothing more is asked this time.
    stopped: bool,
}

/// Sums up `ids` of `journal` that need it, in order, until `limit` are settled, writing is
/// paused, or the model stops answering.
async fn write(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    journal: &Journal,
    ids: &[i64],
    language: &str,
    limit: usize,
) -> Result<Run, DbError> {
    let mut run = Run::default();
    for id in ids {
        if run.settled >= limit || control.paused() {
            break;
        }
        let Some(quest) = journal.quests.get(id) else {
            continue;
        };
        if !wanted(&db.lock().unwrap(), *id, quest, language)? {
            continue;
        }
        match ask(db, provider, control, *id, quest, language).await? {
            Asked::Settled(kept) => {
                run.settled += 1;
                run.kept += usize::from(kept);
            }
            Asked::Stopped => {
                run.stopped = true;
                break;
            }
        }
    }
    Ok(run)
}

/// What a character's gists are written from: its journal as narrators may see it, and its
/// prose language.
fn character(
    db: &Database,
    character_id: i64,
    language_fallback: &str,
) -> Result<Option<(Journal, String)>, DbError> {
    let journal = match archive::load_for(db, character_id, Audience::Shared) {
        Ok(Some(journal)) => journal,
        Ok(None) => return Ok(None),
        Err(archive::ArchiveError::Db(e)) => return Err(e),
        Err(_) => return Ok(None),
    };
    let language = db
        .settings()?
        .writing
        .for_character(&db.character_writing(character_id)?)
        .prose_language
        .unwrap_or_else(|| language_fallback.to_string());
    Ok(Some((journal, language)))
}

/// Sums up the quests `day` accepted or handed in, before its entry is written. Returns how many
/// gists were kept.
pub async fn for_day(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    character_id: i64,
    day: &str,
    language_fallback: &str,
) -> Result<usize, DbError> {
    let Some((journal, language)) =
        character(&db.lock().unwrap(), character_id, language_fallback)?
    else {
        return Ok(0);
    };
    let Some(record) = journal.days.get(day).map(DayRecord::of) else {
        return Ok(0);
    };
    let mut ids: Vec<i64> = Vec::new();
    for id in record.turned_in.iter().chain(&record.accepted) {
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    let run = write(db, provider, control, &journal, &ids, &language, usize::MAX).await?;
    Ok(run.kept)
}

/// In idle time: sums up at most `limit` quests of the characters in the app, the most recently
/// touched first. Returns how many were settled, so the caller knows whether to come back: none
/// when nothing is left, or when the model stopped answering.
pub async fn backfill(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    language_fallback: &str,
    limit: usize,
) -> Result<usize, DbError> {
    let characters: Vec<i64> = db
        .lock()
        .unwrap()
        .characters()?
        .into_iter()
        .filter(|c| c.enabled)
        .map(|c| c.id)
        .collect();
    let mut asked = 0;
    for character_id in characters {
        if asked >= limit || control.paused() {
            break;
        }
        let Some((journal, language)) =
            character(&db.lock().unwrap(), character_id, language_fallback)?
        else {
            continue;
        };
        let mut ids: Vec<(&Option<String>, i64)> = journal
            .quests
            .iter()
            .map(|(id, q)| (&q.last, *id))
            .collect();
        ids.sort_by(|a, b| b.cmp(a));
        let ids: Vec<i64> = ids.into_iter().map(|(_, id)| id).collect();
        let run = write(
            db,
            provider,
            control,
            &journal,
            &ids,
            &language,
            limit - asked,
        )
        .await?;
        asked += run.settled;
        if run.stopped {
            return Ok(0);
        }
    }
    Ok(asked)
}
