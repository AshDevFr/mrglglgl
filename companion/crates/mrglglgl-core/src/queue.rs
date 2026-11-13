//! The background writing queue. Every finished day (never today) and every finished week is
//! queued for writing as soon as the app sees it, and written one at a time, oldest first, so
//! opening the app after a few sessions shows those days already written.
//!
//! The queue lives in the database and survives restarts. A job that fails is retried with a
//! growing pause, three attempts in all, then marked failed with its reason for the player to
//! retry by hand. A job the player cancels is marked failed the same way, without counting as
//! an attempt, so it does not simply start again.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use mrglglgl_llm::{
    CancellationToken, DEFAULT_TEMPERATURE, ENDPOINT_TIMEOUT, LOCAL_TIMEOUT, Provider,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, mpsc};

use crate::archive;
use crate::beats::Thresholds;
use crate::continuity::{self, ContinuitySheet};
use crate::db::{Database, DbError};
use crate::milestones;
use crate::narrators;
use crate::prompt::{self, DayRequest, WeekRequest};
use crate::schema::Journal;
use crate::stories::{self, NewStory, StoryKind, StoryStatus};
use crate::usage::Audience;
use crate::writer::{self, StreamEvent, WriteOptions, WriteResult};

/// Attempts before a job is marked failed.
pub const MAX_ATTEMPTS: i64 = 3;
/// The pause before the first retry; each later one is four times longer.
pub const FIRST_BACKOFF_SECS: i64 = 30;
/// Tokens a weekly chapter may use.
const WEEK_TOKENS: u32 = 800;

/// What the writer is told about a day still going, so the entry does not close the day.
const SO_FAR_NOTE: &str = "The day is not over yet: tell it so far, without ending it.";

/// What the player sees for a job they cancelled.
pub const CANCELLED: &str = "cancelled";
/// What the player sees for a job that ran out of time.
pub const TIMED_OUT: &str = "timedOut";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Pending,
    Running,
    Failed,
}

impl JobState {
    fn key(self) -> &'static str {
        match self {
            JobState::Pending => "pending",
            JobState::Running => "running",
            JobState::Failed => "failed",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "running" => JobState::Running,
            "failed" => JobState::Failed,
            _ => JobState::Pending,
        }
    }
}

/// One day or week waiting to be written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub character_id: i64,
    pub kind: StoryKind,
    pub start_day: String,
    pub state: JobState,
    pub attempts: i64,
    /// Why the last attempt failed, in plain words, or [`CANCELLED`] or [`TIMED_OUT`].
    pub last_error: Option<String>,
    /// A one-off narrator for this entry.
    pub narrator: Option<String>,
    /// The result waits as a draft for the player instead of going into the journal.
    pub draft: bool,
    /// What the player asked of a rewrite.
    pub note: Option<String>,
    /// Tells the day so far, while it is still going.
    pub partial: bool,
    /// Place in the line among the jobs waiting, from 1; `None` when failed or running.
    pub position: Option<u32>,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// The order jobs run in: asked-for first, then oldest first, a week after its own Sunday.
/// A pending week whose own days are still in line, waiting for a retry included: its chapter is
/// told from their entries, so it waits for them. A day that gave up for good holds it no longer.
const WEEK_WAITS: &str = "(kind = 'week' AND EXISTS (
    SELECT 1 FROM write_queue AS day
    WHERE day.character_id = write_queue.character_id AND day.kind = 'day'
      AND day.state IN ('pending', 'running')
      AND day.start_day BETWEEN write_queue.start_day AND date(write_queue.start_day, '+6 days')))";

/// Jobs of characters the player took out of the app: they keep their place, but wait unwritten
/// and unlisted until the character is back.
const OUT_OF_APP: &str = "character_id IN (SELECT id FROM characters WHERE enabled = 0)";

const ORDER: &str = "priority DESC,
    CASE kind WHEN 'week' THEN date(start_day, '+6 days') ELSE start_day END,
    kind";

fn read_job(row: &rusqlite::Row) -> rusqlite::Result<Job> {
    Ok(Job {
        character_id: row.get(0)?,
        kind: StoryKind::from_key(&row.get::<_, String>(1)?).unwrap_or(StoryKind::Day),
        start_day: row.get(2)?,
        state: JobState::from_key(&row.get::<_, String>(3)?),
        attempts: row.get(4)?,
        last_error: row.get(5)?,
        narrator: row.get(6)?,
        draft: row.get(7)?,
        note: row.get(8)?,
        partial: row.get(9)?,
        position: None,
    })
}

const JOB_COLUMNS: &str =
    "character_id, kind, start_day, state, attempts, last_error, narrator, draft, note, partial";

/// Adds a job unless it is already queued.
fn add(
    db: &Database,
    character_id: i64,
    kind: StoryKind,
    start_day: &str,
    draft: bool,
) -> Result<bool, DbError> {
    Ok(db.conn().execute(
        "INSERT OR IGNORE INTO write_queue (character_id, kind, start_day, state, draft)
         VALUES (?1, ?2, ?3, 'pending', ?4)",
        params![character_id, kind.key(), start_day, draft],
    )? > 0)
}

/// Every finished day of the journal and every finished week with days in it, oldest first,
/// each week after its own days. `today` is never finished, nor is its week.
/// The finished days, and the finished weeks when `weekly` chapters are on.
pub fn finished_slots(journal: &Journal, today: &str, weekly: bool) -> Vec<(StoryKind, String)> {
    let mut slots = Vec::new();
    let mut weeks = BTreeMap::new();
    for day in journal.days.keys().filter(|d| d.as_str() < today) {
        slots.push((StoryKind::Day, day.clone()));
        if let Some(monday) = prompt::week_start(day) {
            weeks.insert(monday, ());
        }
    }
    for monday in weeks.into_keys().filter(|_| weekly) {
        let sunday = prompt::week_days(&monday).pop().unwrap_or_default();
        if sunday.as_str() < today {
            slots.push((StoryKind::Week, monday));
        }
    }
    slots
}

/// Queues every finished day and week that has no story yet. A draft or a discarded draft
/// counts as a story: the player decides what happens to those days. In review mode the jobs
/// write drafts. Returns how many jobs were added.
pub fn enqueue_finished(
    db: &Database,
    character_id: i64,
    journal: &Journal,
    today: &str,
) -> Result<usize, DbError> {
    let written = stories::slots(db, character_id)?;
    let writing = db.settings()?.writing;
    let draft = writing.review;
    let mut added = 0;
    for (kind, start_day) in finished_slots(journal, today, writing.weekly) {
        if !written.contains(&(kind, start_day.clone()))
            && add(db, character_id, kind, &start_day, draft)?
        {
            added += 1;
        }
    }
    if added > 0 {
        tracing::info!("character {character_id}: {added} finished days or weeks queued");
    }
    Ok(added)
}

/// How the player asks for a day or week to be written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ask<'a> {
    /// A one-off narrator; `None` uses the chosen one.
    pub narrator: Option<&'a str>,
    /// The result waits as a draft instead of going into the journal.
    pub draft: bool,
    /// What the player asks of this version, for a rewrite.
    pub note: Option<&'a str>,
    /// Tell the day so far, while it is still going: shown on the page at once, and never in
    /// place of the full entry.
    pub partial: bool,
}

/// Queues a day or week to be written (again) ahead of the backlog. A failed job is given a
/// fresh start.
pub fn request(
    db: &Database,
    character_id: i64,
    kind: StoryKind,
    start_day: &str,
    ask: &Ask,
) -> Result<(), DbError> {
    tracing::info!(
        "character {character_id}: {} of {start_day} asked for{}{}{}{}",
        kind.key(),
        if ask.draft { ", as a draft" } else { "" },
        if ask.partial { ", so far" } else { "" },
        ask.narrator
            .map(|n| format!(", narrator {n}"))
            .unwrap_or_default(),
        if ask.note.is_some() {
            ", with a note"
        } else {
            ""
        },
    );
    db.conn().execute(
        "INSERT INTO write_queue
             (character_id, kind, start_day, state, priority, narrator, draft, note, partial)
         VALUES (?1, ?2, ?3, 'pending', 1, ?4, ?5, ?6, ?7)
         ON CONFLICT (character_id, kind, start_day) DO UPDATE SET
             state = CASE state WHEN 'running' THEN 'running' ELSE 'pending' END,
             attempts = 0, last_error = NULL, not_before = 0, priority = 1,
             narrator = excluded.narrator, draft = excluded.draft, note = excluded.note,
             partial = excluded.partial",
        params![
            character_id,
            kind.key(),
            start_day,
            ask.narrator,
            ask.draft,
            ask.note,
            ask.partial
        ],
    )?;
    Ok(())
}

/// Drops every day and week waiting in the queue, for every character; the one being written
/// finishes, and failed ones stay for the player to retry or leave. Finished days with no entry
/// are queued again on the next read of the journal, so only rewrites the player asked for are
/// gone for good. Returns how many were dropped.
pub fn drop_waiting(db: &Database) -> Result<usize, DbError> {
    let dropped = db
        .conn()
        .execute("DELETE FROM write_queue WHERE state = 'pending'", [])?;
    tracing::info!("dropped {dropped} entries waiting to be written");
    Ok(dropped)
}

/// Drops every week waiting or failed in the queue, for every character, once weekly chapters
/// are turned off; one being written finishes. Returns how many were dropped.
pub fn drop_weeks(db: &Database) -> Result<usize, DbError> {
    let dropped = db.conn().execute(
        "DELETE FROM write_queue WHERE kind = 'week' AND state != 'running'",
        [],
    )?;
    if dropped > 0 {
        tracing::info!("weekly chapters off: {dropped} waiting chapters dropped");
    }
    Ok(dropped)
}

/// Puts a failed job back in line, for a retry by hand.
pub fn retry(
    db: &Database,
    character_id: i64,
    kind: StoryKind,
    start_day: &str,
) -> Result<(), DbError> {
    tracing::info!(
        "character {character_id}: {} of {start_day} retried",
        kind.key()
    );
    db.conn().execute(
        "UPDATE write_queue SET state = 'pending', attempts = 0, last_error = NULL, not_before = 0
         WHERE character_id = ?1 AND kind = ?2 AND start_day = ?3 AND state = 'failed'",
        params![character_id, kind.key(), start_day],
    )?;
    Ok(())
}

/// After a restart nothing is running: interrupted jobs go back in line. Chapters queued before
/// weekly chapters were turned off (or before they were off by default) are dropped.
pub fn recover(db: &Database) -> Result<(), DbError> {
    db.conn().execute(
        "UPDATE write_queue SET state = 'pending' WHERE state = 'running'",
        [],
    )?;
    if !db.settings()?.writing.weekly {
        drop_weeks(db)?;
    }
    Ok(())
}

/// The next job that is due, marked as running.
pub fn take_next(db: &Database, now: i64) -> Result<Option<Job>, DbError> {
    let job = db
        .conn()
        .query_row(
            &format!(
                "SELECT {JOB_COLUMNS} FROM write_queue
                 WHERE state = 'pending' AND not_before <= ?1 AND NOT {WEEK_WAITS}
                   AND NOT {OUT_OF_APP}
                 ORDER BY {ORDER} LIMIT 1"
            ),
            [now],
            read_job,
        )
        .optional()?;
    if let Some(job) = &job {
        set_state(db, job, JobState::Running)?;
    }
    Ok(job.map(|j| Job {
        state: JobState::Running,
        ..j
    }))
}

/// Seconds until the next pending job is due, if any is waiting on a retry pause.
pub fn next_due_in(db: &Database, now: i64) -> Result<Option<i64>, DbError> {
    Ok(db
        .conn()
        .query_row(
            &format!(
                "SELECT min(not_before) FROM write_queue
                 WHERE state = 'pending' AND NOT {WEEK_WAITS} AND NOT {OUT_OF_APP}"
            ),
            [],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .map(|at| (at - now).max(0)))
}

fn set_state(db: &Database, job: &Job, state: JobState) -> Result<(), DbError> {
    db.conn().execute(
        "UPDATE write_queue SET state = ?4
         WHERE character_id = ?1 AND kind = ?2 AND start_day = ?3",
        params![job.character_id, job.kind.key(), job.start_day, state.key()],
    )?;
    Ok(())
}

/// Puts back in line the chapter of the week holding `day`, when one was told: it was written
/// without that day's entry, from its notes alone, and is told again with it.
fn retell_week(db: &Database, character_id: i64, day: &str) -> Result<(), DbError> {
    let Some(monday) = prompt::week_start(day) else {
        return Ok(());
    };
    let told = stories::versions(db, character_id, StoryKind::Week, &monday)?
        .iter()
        .any(|s| s.status != StoryStatus::Discarded);
    if told && db.settings()?.writing.weekly {
        tracing::info!("character {character_id}: week of {monday} told again with {day}'s entry");
        let draft = db.settings()?.writing.review;
        add(db, character_id, StoryKind::Week, &monday, draft)?;
    }
    Ok(())
}

fn complete(db: &Database, job: &Job) -> Result<(), DbError> {
    db.conn().execute(
        "DELETE FROM write_queue WHERE character_id = ?1 AND kind = ?2 AND start_day = ?3",
        params![job.character_id, job.kind.key(), job.start_day],
    )?;
    Ok(())
}

/// Records a failed attempt: back in line after a pause, or failed for good after the last.
pub fn fail(db: &Database, job: &Job, reason: &str, now: i64) -> Result<(), DbError> {
    let attempts = job.attempts + 1;
    let (state, not_before) = if attempts >= MAX_ATTEMPTS {
        (JobState::Failed, 0)
    } else {
        (
            JobState::Pending,
            now + FIRST_BACKOFF_SECS * 4_i64.pow(attempts as u32 - 1),
        )
    };
    if state == JobState::Failed {
        tracing::warn!(
            "character {}: {} of {} failed for good after {attempts} attempts: {reason}",
            job.character_id,
            job.kind.key(),
            job.start_day
        );
    } else {
        tracing::warn!(
            "character {}: {} of {} failed (attempt {attempts} of {MAX_ATTEMPTS}), tried again in {}s: {reason}",
            job.character_id,
            job.kind.key(),
            job.start_day,
            not_before - now
        );
    }
    db.conn().execute(
        "UPDATE write_queue SET state = ?4, attempts = ?5, last_error = ?6, not_before = ?7,
             priority = 0
         WHERE character_id = ?1 AND kind = ?2 AND start_day = ?3",
        params![
            job.character_id,
            job.kind.key(),
            job.start_day,
            state.key(),
            attempts,
            reason,
            not_before
        ],
    )?;
    Ok(())
}

/// Sets a job aside without counting an attempt: the player cancelled it.
fn set_aside(db: &Database, job: &Job, reason: &str) -> Result<(), DbError> {
    db.conn().execute(
        "UPDATE write_queue SET state = 'failed', last_error = ?4, priority = 0
         WHERE character_id = ?1 AND kind = ?2 AND start_day = ?3",
        params![job.character_id, job.kind.key(), job.start_day, reason],
    )?;
    Ok(())
}

/// A character's jobs, in the order they will run, failed ones last.
pub fn jobs(db: &Database, character_id: i64) -> Result<Vec<Job>, DbError> {
    Ok(all_jobs(db)?
        .into_iter()
        .filter(|job| job.character_id == character_id)
        .collect())
}

/// Every character's jobs, in the order they will run, failed ones last.
pub fn all_jobs(db: &Database) -> Result<Vec<Job>, DbError> {
    let mut stmt = db.conn().prepare(&format!(
        "SELECT {JOB_COLUMNS} FROM write_queue
         WHERE NOT {OUT_OF_APP}
         ORDER BY state = 'failed', state != 'running', {ORDER}"
    ))?;
    let all: Vec<Job> = stmt.query_map([], read_job)?.collect::<Result<_, _>>()?;
    // Positions count every character's waiting jobs: they share one writer.
    let mut position = 0;
    Ok(all
        .into_iter()
        .map(|mut job| {
            if job.state == JobState::Pending {
                position += 1;
                job.position = Some(position);
            }
            job
        })
        .collect())
}

/// What the queue tells whoever is watching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum QueueEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        character_id: i64,
        kind: StoryKind,
        start_day: String,
        /// The narrator writing it, so the window can show who is at work.
        narrator: String,
    },
    #[serde(rename_all = "camelCase")]
    Chunk {
        character_id: i64,
        kind: StoryKind,
        start_day: String,
        text: String,
    },
    /// The fact check sent the entry back; the text so far is discarded.
    #[serde(rename_all = "camelCase")]
    Restarted {
        character_id: i64,
        kind: StoryKind,
        start_day: String,
    },
    #[serde(rename_all = "camelCase")]
    Finished {
        character_id: i64,
        kind: StoryKind,
        start_day: String,
        /// `"written"`, `"setAside"` (the fact check failed), `"cancelled"`, `"timedOut"` or
        /// `"failed"`.
        outcome: String,
        message: Option<String>,
    },
}

/// A narrator preview the player asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct PreviewStatus {
    /// The narrator's name, as the player sees it.
    pub narrator: String,
    /// It has its turn; until then it waits for the entry being written.
    pub running: bool,
}

/// Shared between the writer loop and the commands that steer it.
#[derive(Default)]
pub struct Control {
    paused: AtomicBool,
    /// Where the player's own narrators are kept; built-ins only when `None`.
    narrators: Option<PathBuf>,
    /// Taken for every generation, entry or preview: one model, one writer at a time. A preview
    /// waits for the entry being written, then goes before the next one.
    writer: tokio::sync::Mutex<()>,
    /// The preview asked for, waiting or being written, and how to stop it.
    preview: Mutex<Option<(PreviewStatus, CancellationToken)>>,
    wake: Notify,
    current: Mutex<Option<(Job, CancellationToken)>>,
}

impl Control {
    pub fn new(paused: bool) -> Self {
        Self {
            paused: AtomicBool::new(paused),
            ..Self::default()
        }
    }

    /// Reads the player's own narrators from `dir` whenever an entry is prepared, so an edit
    /// applies to the next entry.
    pub fn with_narrators(mut self, dir: PathBuf) -> Self {
        self.narrators = Some(dir);
        self
    }

    /// The narrators to write with: the built-ins and the player's own.
    pub fn narrators(&self) -> narrators::Library {
        self.narrators
            .as_deref()
            .map(narrators::Library::load)
            .unwrap_or_default()
    }

    /// Pausing lets the current entry finish and starts no other.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
        self.wake();
    }

    /// Mirrors the saved setting without waking anyone, for the loop that reads it.
    pub fn set_paused_quietly(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Something changed (a job was queued, a setting saved): look at the queue again.
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    /// The job being written now, if any.
    pub fn current(&self) -> Option<Job> {
        self.current
            .lock()
            .unwrap()
            .as_ref()
            .map(|(job, _)| job.clone())
    }

    /// The preview asked for, if any: waiting for its turn or being written.
    pub fn preview_status(&self) -> Option<PreviewStatus> {
        self.preview
            .lock()
            .unwrap()
            .as_ref()
            .map(|(status, _)| status.clone())
    }

    /// Stops the preview asked for, waiting or being written. Returns whether there was one.
    pub fn cancel_preview(&self) -> bool {
        match self.preview.lock().unwrap().as_ref() {
            Some((_, token)) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// Cancels the entry being written, if it is the one named (any one when `None`).
    pub fn cancel(&self, which: Option<(i64, StoryKind, &str)>) -> bool {
        let current = self.current.lock().unwrap();
        match current.as_ref() {
            Some((job, token))
                if which.is_none_or(|(c, k, d)| {
                    job.character_id == c && job.kind == k && job.start_day == d
                }) =>
            {
                token.cancel();
                true
            }
            _ => false,
        }
    }
}

/// The prompt the next write of this day or week would get, built as the writer builds it: with
/// the narrator, language and preferences in the settings now.
pub fn next_prompt(
    db: &Database,
    library: &narrators::Library,
    character_id: i64,
    kind: StoryKind,
    start_day: &str,
    language_fallback: &str,
    provider_is_endpoint: bool,
) -> Result<Result<prompt::Prompt, String>, DbError> {
    let job = Job {
        character_id,
        kind,
        start_day: start_day.to_string(),
        state: JobState::Pending,
        attempts: 0,
        last_error: None,
        narrator: None,
        draft: false,
        note: None,
        partial: false,
        position: None,
    };
    Ok(prepare(db, library, &job, language_fallback, provider_is_endpoint)?.map(|p| p.prompt))
}

/// Everything a job needs, gathered under one short lock.
struct Prepared {
    prompt: prompt::Prompt,
    narrator: String,
    language: String,
    end_day: String,
    allowed: Vec<String>,
    journal: Journal,
    check_retries: u32,
}

fn prepare(
    db: &Database,
    library: &narrators::Library,
    job: &Job,
    language_fallback: &str,
    provider_is_endpoint: bool,
) -> Result<Result<Prepared, String>, DbError> {
    let settings = db
        .settings()?
        .writing
        .for_character(&db.character_writing(job.character_id)?);
    // The narrator is told only what the player lets the app use for this character.
    let journal = match archive::load_for(db, job.character_id, Audience::Shared) {
        Ok(Some(journal)) => journal,
        Ok(None) => return Ok(Err("the character's journal is gone".to_string())),
        Err(archive::ArchiveError::Db(e)) => return Err(e),
        Err(e) => return Ok(Err(format!("the journal could not be read: {e}"))),
    };
    let unused = db.hidden(job.character_id, Audience::Shared)?;
    let awards =
        crate::usage::awards_in_use(milestones::awards(db, job.character_id)?, &journal, &unused);
    let jokes = continuity::load(db, job.character_id)?
        .map(|s| s.running_jokes)
        .unwrap_or_default();
    let language = settings
        .prose_language
        .clone()
        .unwrap_or_else(|| language_fallback.to_string());
    let narrator_id = job
        .narrator
        .clone()
        .unwrap_or_else(|| settings.narrator.clone());
    let narrator = library.get(&narrator_id, &language);
    let pseudonyms = provider_is_endpoint && settings.pseudonyms;
    let thresholds = Thresholds::default();
    let (prompt, end_day) = match job.kind {
        StoryKind::Day => (
            prompt::day_prompt(&DayRequest {
                character_id: job.character_id,
                journal: &journal,
                day: &job.start_day,
                awards: &awards,
                running_jokes: &jokes,
                narrator,
                language: &language,
                meddling: settings.meddling,
                pseudonyms,
                thresholds: &thresholds,
                note: if job.partial {
                    Some(job.note.as_deref().unwrap_or(SO_FAR_NOTE))
                } else {
                    job.note.as_deref()
                },
                preferences: Some(&settings.preferences),
            }),
            job.start_day.clone(),
        ),
        StoryKind::Week => {
            let entries = stories::day_entries(db, job.character_id)?;
            (
                prompt::week_prompt(&WeekRequest {
                    character_id: job.character_id,
                    journal: &journal,
                    start_day: &job.start_day,
                    awards: &awards,
                    running_jokes: &jokes,
                    entries: &entries,
                    narrator,
                    language: &language,
                    meddling: settings.meddling,
                    pseudonyms,
                    thresholds: &thresholds,
                    note: job.note.as_deref(),
                    preferences: Some(&settings.preferences),
                }),
                prompt::week_days(&job.start_day).pop().unwrap_or_default(),
            )
        }
    };
    let allowed = allowed_words(&journal, narrator);
    Ok(Ok(Prepared {
        prompt,
        narrator: narrator.id.clone(),
        language,
        end_day,
        allowed,
        journal,
        check_retries: settings.check_retries.min(crate::db::MAX_CHECK_RETRIES),
    }))
}

/// Words an entry may use besides the notes: the character's faction and realm, and the
/// narrator's own framing.
fn allowed_words(journal: &Journal, narrator: &narrators::Narrator) -> Vec<String> {
    let c = &journal.character;
    [&c.faction, &c.realm]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .chain(narrator.own_words())
        .map(str::to_string)
        .collect()
}

fn plain(e: &mrglglgl_llm::ProviderError) -> String {
    e.to_string()
}

/// Takes the next due job and writes it. Returns `None` when nothing is due. Errors are the
/// database's; a provider failure is recorded on the job.
pub async fn run_next(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    language_fallback: &str,
    on_event: &(dyn Fn(QueueEvent) + Send + Sync),
) -> Result<Option<Job>, DbError> {
    let is_endpoint = provider.id() == "endpoint";
    let (job, prepared) = {
        let db = db.lock().unwrap();
        let Some(job) = take_next(&db, now())? else {
            return Ok(None);
        };
        let prepared = prepare(
            &db,
            &control.narrators(),
            &job,
            language_fallback,
            is_endpoint,
        )?;
        (job, prepared)
    };
    let prepared = match prepared {
        Ok(p) => p,
        Err(reason) => {
            tracing::warn!(
                "character {}: {} of {} could not be prepared: {reason}",
                job.character_id,
                job.kind.key(),
                job.start_day
            );
            fail(&db.lock().unwrap(), &job, &reason, now())?;
            return Ok(Some(job));
        }
    };

    let _turn = control.writer.lock().await;
    let cancel = CancellationToken::new();
    *control.current.lock().unwrap() = Some((job.clone(), cancel.clone()));
    let (character_id, kind, start_day) = (job.character_id, job.kind, job.start_day.clone());
    on_event(QueueEvent::Started {
        character_id,
        kind,
        start_day: start_day.clone(),
        narrator: prepared.narrator.clone(),
    });

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let forward = async {
        while let Some(event) = rx.recv().await {
            on_event(match event {
                StreamEvent::Chunk(text) => QueueEvent::Chunk {
                    character_id,
                    kind,
                    start_day: start_day.clone(),
                    text,
                },
                StreamEvent::Restart => QueueEvent::Restarted {
                    character_id,
                    kind,
                    start_day: start_day.clone(),
                },
            });
        }
    };
    let options = WriteOptions {
        language: prepared.language.clone(),
        max_tokens: match kind {
            StoryKind::Day => prompt::tokens_for(prepared.prompt.words),
            StoryKind::Week => WEEK_TOKENS,
        },
        temperature: DEFAULT_TEMPERATURE,
        timeout: if is_endpoint {
            ENDPOINT_TIMEOUT
        } else {
            LOCAL_TIMEOUT
        },
        allowed: prepared.allowed.clone(),
        check_retries: prepared.check_retries,
    };
    let write = async {
        let result = writer::write(provider, &prepared.prompt, &options, &cancel, Some(&tx)).await;
        drop(tx);
        result
    };
    tracing::info!(
        "character {character_id}: writing {} of {start_day} with {} ({}), narrator {}, in {}, \
         aiming for {} words in at most {} tokens{}",
        kind.key(),
        provider.id(),
        provider.model(),
        prepared.narrator,
        prepared.language,
        prepared.prompt.words,
        options.max_tokens,
        if job.draft { ", as a draft" } else { "" },
    );
    tracing::debug!(
        "prompt {}: {} system and {} user characters",
        prepared.prompt.hash,
        prepared.prompt.system.len(),
        prepared.prompt.user.len()
    );
    tracing::trace!(
        "system prompt:\n{}\nuser prompt:\n{}",
        prepared.prompt.system,
        prepared.prompt.user
    );
    let started = std::time::Instant::now();
    let (result, ()) = tokio::join!(write, forward);
    *control.current.lock().unwrap() = None;
    let took = started.elapsed();

    let db = db.lock().unwrap();
    let (outcome, message) = match result {
        Ok(WriteResult::Written {
            text,
            check,
            reason,
        }) => {
            let status = if job.draft && !job.partial {
                StoryStatus::Draft
            } else {
                StoryStatus::Accepted
            };
            // The day's first entry in the journal, after its week's chapter was told without it.
            let first_entry = kind == StoryKind::Day
                && !job.partial
                && status == StoryStatus::Accepted
                && !stories::versions(&db, character_id, kind, &start_day)?
                    .iter()
                    .any(|s| s.status == StoryStatus::Accepted && !s.partial);
            stories::save(
                &db,
                &NewStory {
                    character_id,
                    kind,
                    start_day: start_day.clone(),
                    end_day: prepared.end_day.clone(),
                    narrator: prepared.narrator.clone(),
                    language: prepared.language.clone(),
                    provider: provider.id().to_string(),
                    model: provider.model().to_string(),
                    prompt_hash: prepared.prompt.hash.clone(),
                    text,
                    check,
                    check_reason: reason.clone(),
                    // A day told so far is asked for to be read now, so it is never a draft.
                    status,
                    note: job.note.clone(),
                    partial: job.partial,
                },
            )?;
            let unused = db.hidden(character_id, Audience::Shared)?;
            let awards = crate::usage::awards_in_use(
                milestones::awards(&db, character_id)?,
                &prepared.journal,
                &unused,
            );
            continuity::refresh(&db, character_id, &prepared.journal, &awards, None)?;
            complete(&db, &job)?;
            if first_entry {
                retell_week(&db, character_id, &start_day)?;
            }
            match reason {
                Some(r) => {
                    tracing::warn!(
                        "character {character_id}: {} of {start_day} set aside after {took:?}: {r}",
                        kind.key()
                    );
                    ("setAside", Some(r))
                }
                None => {
                    tracing::info!(
                        "character {character_id}: {} of {start_day} written in {took:?}{}",
                        kind.key(),
                        if first_entry {
                            "; its week is told again"
                        } else {
                            ""
                        }
                    );
                    ("written", None)
                }
            }
        }
        Ok(WriteResult::Cancelled) => {
            tracing::info!(
                "character {character_id}: {} of {start_day} cancelled after {took:?}",
                kind.key()
            );
            set_aside(&db, &job, CANCELLED)?;
            ("cancelled", None)
        }
        Ok(WriteResult::TimedOut) => {
            tracing::debug!(
                "character {character_id}: {} of {start_day} timed out after {took:?}",
                kind.key()
            );
            fail(&db, &job, TIMED_OUT, now())?;
            ("timedOut", None)
        }
        Err(e) => {
            let message = plain(&e);
            fail(&db, &job, &message, now())?;
            ("failed", Some(message))
        }
    };
    on_event(QueueEvent::Finished {
        character_id,
        kind,
        start_day,
        outcome: outcome.to_string(),
        message,
    });
    Ok(Some(job))
}

/// Writes due jobs one at a time until none is due or the queue is paused. Returns how many ran.
pub async fn drain(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    language_fallback: &str,
    on_event: &(dyn Fn(QueueEvent) + Send + Sync),
) -> Result<usize, DbError> {
    let mut ran = 0;
    while !control.paused() {
        match run_next(db, provider, control, language_fallback, on_event).await? {
            Some(_) => ran += 1,
            None => break,
        }
    }
    Ok(ran)
}

/// Waits until something may have changed: a wake-up, or `timeout`.
pub async fn idle(control: &Control, timeout: Duration) {
    let _ = tokio::time::timeout(timeout, control.wake.notified()).await;
}

/// The sheet the next entry would be written with, for display.
pub fn sheet(
    db: &Database,
    character_id: i64,
    journal: &Journal,
) -> Result<ContinuitySheet, DbError> {
    let unused = db.hidden(character_id, Audience::Shared)?;
    let awards =
        crate::usage::awards_in_use(milestones::awards(db, character_id)?, journal, &unused);
    let jokes = continuity::load(db, character_id)?
        .map(|s| s.running_jokes)
        .unwrap_or_default();
    Ok(ContinuitySheet::compute(journal, &awards, &jokes, None))
}

/// The day a narrator preview writes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PreviewDay {
    /// One of a character's days.
    #[serde(rename_all = "camelCase")]
    Character { character_id: i64, day: String },
    /// The built-in sample day, for a player with no journal yet.
    Sample,
}

/// A made-up adventurer's journal and its liveliest day, for previews without a journal.
const SAMPLE_JOURNAL: &str =
    include_str!("../../../../fixtures/savedvariables/synthetic-character.lua");
const SAMPLE_DAY: &str = "2026-11-08";

/// Writes one day with `narrator`, saved or not, the way a real entry would be written (fact
/// check included), and stores nothing. It takes its turn with the background writer.
/// `on_event` receives the text as it streams. Errors are plain sentences.
pub async fn preview(
    db: &Mutex<Database>,
    provider: &dyn Provider,
    control: &Control,
    language_fallback: &str,
    narrator: &narrators::Narrator,
    day: &PreviewDay,
    on_event: &(dyn Fn(StreamEvent) + Send + Sync),
) -> Result<WriteResult, String> {
    let is_endpoint = provider.id() == "endpoint";
    let (prompt, language, allowed, check_retries) = {
        let db = db.lock().unwrap();
        let settings = db.settings().map_err(|e| e.to_string())?.writing;
        // A day of a real character is previewed with that character's margin notes and
        // preferences; the narrator is the one being previewed.
        let settings = match day {
            PreviewDay::Character { character_id, .. } => settings.for_character(
                &db.character_writing(*character_id)
                    .map_err(|e| e.to_string())?,
            ),
            PreviewDay::Sample => settings,
        };
        let language = settings
            .prose_language
            .clone()
            .unwrap_or_else(|| language_fallback.to_string());
        let (character_id, journal, date, awards, jokes) = match day {
            PreviewDay::Character { character_id, day } => {
                let journal = archive::load_for(&db, *character_id, Audience::Shared)
                    .map_err(|e| e.to_string())?
                    .ok_or("this character has no journal yet")?;
                let awards = crate::usage::awards(&db, *character_id, Audience::Shared)
                    .map_err(|e| e.to_string())?;
                let jokes = continuity::load(&db, *character_id)
                    .map_err(|e| e.to_string())?
                    .map(|s| s.running_jokes)
                    .unwrap_or_default();
                (*character_id, journal, day.clone(), awards, jokes)
            }
            PreviewDay::Sample => {
                let journal = crate::schema::journal_from_text(SAMPLE_JOURNAL)
                    .expect("the sample journal reads");
                let awards = milestones::compute(&journal);
                (0, journal, SAMPLE_DAY.to_string(), awards, Vec::new())
            }
        };
        let prompt = prompt::day_prompt(&DayRequest {
            character_id,
            journal: &journal,
            day: &date,
            awards: &awards,
            running_jokes: &jokes,
            narrator,
            language: &language,
            meddling: settings.meddling,
            pseudonyms: is_endpoint && settings.pseudonyms,
            thresholds: &Thresholds::default(),
            note: None,
            preferences: Some(&settings.preferences),
        });
        let allowed = allowed_words(&journal, narrator);
        let retries = settings.check_retries.min(crate::db::MAX_CHECK_RETRIES);
        (prompt, language, allowed, retries)
    };

    let cancel = CancellationToken::new();
    *control.preview.lock().unwrap() = Some((
        PreviewStatus {
            narrator: narrator.name.clone(),
            running: false,
        },
        cancel.clone(),
    ));
    // However the preview ends, it is no longer asked for.
    let _asked = PreviewAsked(control);
    // The player may stop it while it still waits for the entry being written.
    let _turn = tokio::select! {
        turn = control.writer.lock() => turn,
        () = cancel.cancelled() => return Ok(WriteResult::Cancelled),
    };
    if let Some((status, _)) = control.preview.lock().unwrap().as_mut() {
        status.running = true;
    }
    let options = WriteOptions {
        language,
        max_tokens: prompt::tokens_for(prompt.words),
        temperature: DEFAULT_TEMPERATURE,
        timeout: if is_endpoint {
            ENDPOINT_TIMEOUT
        } else {
            LOCAL_TIMEOUT
        },
        allowed,
        check_retries,
    };
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let forward = async {
        while let Some(event) = rx.recv().await {
            on_event(event);
        }
    };
    let write = async {
        let result = writer::write(provider, &prompt, &options, &cancel, Some(&tx)).await;
        drop(tx);
        result
    };
    let (result, ()) = tokio::join!(write, forward);
    result.map_err(|e| plain(&e))
}

/// Clears the preview from [`Control`] when it ends, however it ends.
struct PreviewAsked<'a>(&'a Control);

impl Drop for PreviewAsked<'_> {
    fn drop(&mut self) {
        *self.0.preview.lock().unwrap() = None;
    }
}
