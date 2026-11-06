//! Backups of each journal file and the archive built from them.
//!
//! The addon keeps raw events for a few days and then compacts them to a digest. The app keeps
//! everything it has ever seen: every change to a journal is copied into the app data folder
//! (never next to the original), the copy is parsed (the game may be rewriting the original at
//! that moment), and the result is merged into the archive day by day.
//!
//! Merge rule, per day: the newer snapshot wins, except that raw events are never replaced by
//! their absence. A day the addon has since compacted keeps the raw events an earlier snapshot
//! had; an older snapshot read late only fills in what is missing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::{BackupRetention, Database, DbError};
use crate::discovery::CharacterSource;
use crate::schema::{
    self, AddonErrors, AddonHeartbeat, Character, Companion, Day, Journal, JournalError, Quest,
    Stats,
};
use crate::watch::{modified_unix, now_unix};

/// The folder, inside the app data folder, that holds journal backups.
pub const BACKUPS_FOLDER: &str = "backups";

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("file error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn io_error(path: &Path) -> impl FnOnce(io::Error) -> ArchiveError + '_ {
    move |source| ArchiveError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// The folder holding one character's backups.
pub fn backup_folder(data_dir: &Path, source: &CharacterSource) -> PathBuf {
    data_dir
        .join(BACKUPS_FOLDER)
        .join(&source.flavor)
        .join(&source.account)
        .join(&source.realm_folder)
        .join(&source.character_folder)
}

/// A character's backups, oldest first. Names start with the zero-padded modification time of
/// the original, so name order is time order.
pub fn backups(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "lua"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Every copy younger than this many days is kept; older ones thin to one per day.
pub const KEEP_EVERY_COPY_DAYS: i64 = 7;

const DAY_SECONDS: i64 = 24 * 60 * 60;

/// When the game wrote the journal a backup copies, from the backup's name.
fn backup_time(path: &Path) -> Option<i64> {
    let stem = path.file_stem()?.to_str()?;
    stem.split('-').next()?.parse().ok()
}

fn local_day(unix: i64) -> Option<chrono::NaiveDate> {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(unix, 0)
        .earliest()
        .map(|t| t.date_naive())
}

/// The backups to delete at time `now`: copies from the last week are all kept; older copies
/// thin to the last one of each local calendar day; copies older than the retention period go.
/// The newest copy is always kept, however old: it holds the journal's current content.
pub fn backups_to_remove(
    backups: &[PathBuf],
    now: i64,
    retention: BackupRetention,
) -> Vec<PathBuf> {
    let mut dated: Vec<(i64, &PathBuf)> = backups
        .iter()
        .filter_map(|p| Some((backup_time(p)?, p)))
        .collect();
    dated.sort();
    let Some(&(_, newest)) = dated.last() else {
        return Vec::new();
    };

    let mut last_of_day: BTreeMap<chrono::NaiveDate, &PathBuf> = BTreeMap::new();
    for (time, path) in &dated {
        if let Some(day) = local_day(*time) {
            last_of_day.insert(day, path);
        }
    }

    dated
        .iter()
        .filter(|(time, path)| {
            if *path == newest {
                return false;
            }
            let age = now - time;
            let expired = retention
                .days()
                .is_some_and(|days| age > days * DAY_SECONDS);
            let thinned = age >= KEEP_EVERY_COPY_DAYS * DAY_SECONDS
                && local_day(*time).and_then(|d| last_of_day.get(&d)) != Some(path);
            expired || thinned
        })
        .map(|(_, path)| (*path).clone())
        .collect()
}

/// Copies a character's journal into its backup folder, unless the newest backup already holds
/// the same content, then thins old copies for the retention period at time `now`. Returns the
/// backup holding the current content, which is the file to parse. The original is only ever
/// read.
pub fn backup_journal(
    data_dir: &Path,
    source: &CharacterSource,
    retention: BackupRetention,
    now: i64,
) -> Result<PathBuf, ArchiveError> {
    let content = fs::read(&source.journal_path).map_err(io_error(&source.journal_path))?;
    let folder = backup_folder(data_dir, source);
    fs::create_dir_all(&folder).map_err(io_error(&folder))?;

    let existing = backups(&folder);
    let path = match existing.last() {
        Some(newest) if fs::read(newest).is_ok_and(|bytes| bytes == content) => newest.clone(),
        _ => {
            let mtime = modified_unix(&source.journal_path).unwrap_or(0);
            let mut path = folder.join(format!("{mtime:012}.lua"));
            let mut n = 1;
            while path.exists() {
                path = folder.join(format!("{mtime:012}-{n}.lua"));
                n += 1;
            }
            fs::write(&path, &content).map_err(io_error(&path))?;
            path
        }
    };

    for old in backups_to_remove(&backups(&folder), now, retention) {
        fs::remove_file(&old).map_err(io_error(&old))?;
    }
    Ok(path)
}

/// Everything in a journal except its days, stored as one record per character.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct JournalRecord {
    schema: i64,
    character: Character,
    quests: BTreeMap<i64, Quest>,
    companions: BTreeMap<String, Companion>,
    discovered: BTreeSet<String>,
    stats: Stats,
    addon: Option<AddonHeartbeat>,
    errors: Option<AddonErrors>,
}

fn merge_quest(into: &mut Quest, from: &Quest) {
    if from.title.is_some() {
        into.title.clone_from(&from.title);
    }
    // The addon drops a quest's text once the quest leaves its retention window; the archive
    // keeps the text it already has, so an absent text never erases one.
    if from.text.is_some() {
        into.text.clone_from(&from.text);
    }
    // Day keys sort as strings, so the later of the two is kept.
    if from.last > into.last {
        into.last.clone_from(&from.last);
    }
}

/// Merges one parsed snapshot of a character's journal into the archive. `source_mtime` is when
/// the game wrote the snapshot; it decides which of two snapshots is newer.
pub fn merge_snapshot(
    db: &Database,
    character_id: i64,
    journal: &Journal,
    source_mtime: i64,
) -> Result<(), ArchiveError> {
    let conn = db.conn();
    let tx = conn.unchecked_transaction()?;

    for (date, incoming) in &journal.days {
        let stored: Option<(String, i64)> = tx
            .query_row(
                "SELECT data_json, source_mtime FROM archive_days
                 WHERE character_id = ?1 AND day = ?2",
                params![character_id, date],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (merged, mtime) = match stored {
            None => (incoming.clone(), source_mtime),
            Some((json, stored_mtime)) => {
                let stored: Day = serde_json::from_str(&json)?;
                if source_mtime >= stored_mtime {
                    let mut merged = incoming.clone();
                    if merged.events.is_none() {
                        merged.events = stored.events;
                        if merged.kills.is_empty() {
                            merged.kills = stored.kills;
                        }
                        if merged.anon_kills == 0 {
                            merged.anon_kills = stored.anon_kills;
                        }
                    }
                    if merged.digest.is_none() {
                        merged.digest = stored.digest;
                    }
                    (merged, source_mtime)
                } else {
                    let mut merged = stored;
                    if merged.events.is_none() && incoming.events.is_some() {
                        merged.events.clone_from(&incoming.events);
                        merged.kills.clone_from(&incoming.kills);
                        merged.anon_kills = incoming.anon_kills;
                    }
                    if merged.digest.is_none() {
                        merged.digest.clone_from(&incoming.digest);
                    }
                    (merged, stored_mtime)
                }
            }
        };
        tx.execute(
            "INSERT INTO archive_days (character_id, day, data_json, source_mtime)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (character_id, day) DO UPDATE SET
                 data_json = excluded.data_json, source_mtime = excluded.source_mtime",
            params![character_id, date, serde_json::to_string(&merged)?, mtime],
        )?;
    }

    let stored: Option<(String, i64)> = tx
        .query_row(
            "SELECT data_json, source_mtime FROM archive_journals WHERE character_id = ?1",
            [character_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (mut record, stored_mtime) = match stored {
        Some((json, mtime)) => (serde_json::from_str::<JournalRecord>(&json)?, mtime),
        None => (JournalRecord::default(), i64::MIN),
    };
    let newer = source_mtime >= stored_mtime;
    if newer {
        record.schema = journal.schema;
        record.character = journal.character.clone();
        record.stats = journal.stats.clone();
        if journal.addon.is_some() {
            record.addon.clone_from(&journal.addon);
        }
        if journal.errors.is_some() {
            record.errors.clone_from(&journal.errors);
        }
    }
    for (id, quest) in &journal.quests {
        let entry = record.quests.entry(*id).or_default();
        if newer {
            merge_quest(entry, quest);
        } else {
            let mut filled = quest.clone();
            merge_quest(&mut filled, entry);
            *entry = filled;
        }
    }
    for (who, companion) in &journal.companions {
        if newer || !record.companions.contains_key(who) {
            record.companions.insert(who.clone(), companion.clone());
        }
    }
    record.discovered.extend(journal.discovered.iter().cloned());
    tx.execute(
        "INSERT INTO archive_journals (character_id, data_json, source_mtime)
         VALUES (?1, ?2, ?3)
         ON CONFLICT (character_id) DO UPDATE SET
             data_json = excluded.data_json, source_mtime = excluded.source_mtime",
        params![
            character_id,
            serde_json::to_string(&record)?,
            stored_mtime.max(source_mtime)
        ],
    )?;
    tx.commit()?;
    Ok(())
}

/// A character's archived journal: every day the app has seen, merged.
pub fn load(db: &Database, character_id: i64) -> Result<Option<Journal>, ArchiveError> {
    let conn = db.conn();
    let record: Option<String> = conn
        .query_row(
            "SELECT data_json FROM archive_journals WHERE character_id = ?1",
            [character_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(record) = record else {
        return Ok(None);
    };
    let record: JournalRecord = serde_json::from_str(&record)?;
    let mut stmt =
        conn.prepare("SELECT day, data_json FROM archive_days WHERE character_id = ?1")?;
    let days = stmt
        .query_map([character_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .map(|row| {
            let (date, json) = row?;
            Ok((date, serde_json::from_str::<Day>(&json)?))
        })
        .collect::<Result<BTreeMap<_, _>, ArchiveError>>()?;
    Ok(Some(Journal {
        schema: record.schema,
        character: record.character,
        days,
        quests: record.quests,
        companions: record.companions,
        discovered: record.discovered,
        stats: record.stats,
        addon: record.addon,
        errors: record.errors,
    }))
}

/// What reading a character's journal produced.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportOutcome {
    /// Read and merged into the archive.
    Imported { character_id: i64 },
    /// Backed up but not readable by this app; the archive keeps what it had. The status is also
    /// recorded on the character for the UI.
    Unreadable {
        character_id: i64,
        error: JournalError,
    },
}

fn status_of(error: &JournalError) -> &'static str {
    match error {
        JournalError::OlderSchema { .. } => "older",
        JournalError::NewerSchema { .. } => "newer",
        JournalError::Parse(_) | JournalError::NoJournal => "unreadable",
    }
}

/// Backs up a character's journal, parses the backup and merges it into the archive. One bad
/// journal never stops another from importing: the error is returned for this character only.
pub fn import(
    db: &Database,
    data_dir: &Path,
    source: &CharacterSource,
    retention: BackupRetention,
) -> Result<ImportOutcome, ArchiveError> {
    let copy = backup_journal(data_dir, source, retention, now_unix())?;
    let text = fs::read_to_string(&copy).map_err(io_error(&copy))?;
    let synced = modified_unix(&source.journal_path);
    match schema::journal_from_text(&text) {
        Ok(journal) => {
            let character_id = db.upsert_character(source, Some(&journal.character), synced)?;
            merge_snapshot(db, character_id, &journal, synced.unwrap_or(0))?;
            db.set_character_status(character_id, None)?;
            Ok(ImportOutcome::Imported { character_id })
        }
        Err(error) => {
            let character_id = db.upsert_character(source, None, synced)?;
            db.set_character_status(character_id, Some(status_of(&error)))?;
            Ok(ImportOutcome::Unreadable {
                character_id,
                error,
            })
        }
    }
}

/// Removes a character from the app: its archive, milestones and backups. The game's own files
/// are not touched.
pub fn delete_character(
    db: &Database,
    data_dir: &Path,
    character_id: i64,
) -> Result<(), ArchiveError> {
    if let Some(character) = db.character(character_id)? {
        let source = CharacterSource {
            flavor: character.flavor,
            account: character.account,
            realm_folder: character.realm_folder,
            character_folder: character.character_folder,
            journal_path: character.journal_path,
        };
        let folder = backup_folder(data_dir, &source);
        if folder.exists() {
            fs::remove_dir_all(&folder).map_err(io_error(&folder))?;
        }
    }
    db.delete_character(character_id)?;
    Ok(())
}
