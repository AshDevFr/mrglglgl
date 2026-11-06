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
    self, AddonErrors, AddonHeartbeat, Character, Companion, Day, Digest, ItemCount, Journal,
    JournalError, Quest, Stats,
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

/// The earlier of two optional day keys.
fn min_some(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// How much a digest holds: one thinned by age always holds less than one that is not; then the
/// entries it lists and the counts it keeps.
fn fullness(digest: &Digest) -> (bool, usize, i64) {
    let d = digest;
    let listed = d.zones.len()
        + d.subzones.len()
        + d.discovered.len()
        + d.accepted.len()
        + d.turned_in.len()
        + d.deaths.len()
        + d.kills.len()
        + d.notable_kills.len()
        + d.fish.len()
        + d.loot.len()
        + d.companions.len()
        + d.bosses.len()
        + d.skills.len()
        + d.crafted.len()
        + d.gathered.len();
    let c = &d.counts;
    let counted = c.sessions
        + c.levels
        + c.accepted
        + c.turned_in
        + c.deaths
        + c.fish
        + c.loot
        + c.discoveries
        + c.bosses
        + c.kills
        + c.skill_ups
        + c.crafted
        + c.gathered;
    (!d.thin, listed, counted)
}

/// One day from what the archive holds and a snapshot, never with less of anything than either:
/// the longer event list, every kill, skill and item with the larger count, the fuller digest.
/// On a tie the newer snapshot's version is kept. The game shrinks a day as it ages (compaction,
/// thinning) and when the player uses Forget; neither reaches the archive, and neither does an old
/// file read late with a fresh modification time.
fn merge_day(stored: Day, incoming: Day, incoming_newer: bool) -> Day {
    let prefer = |stored_len: usize, incoming_len: usize| {
        incoming_len > stored_len || (incoming_len == stored_len && incoming_newer)
    };
    let events = match (stored.events, incoming.events) {
        (Some(s), Some(i)) => Some(if prefer(s.len(), i.len()) { i } else { s }),
        (s, i) => i.or(s),
    };

    let mut kills = stored.kills;
    for (npc, kill) in incoming.kills {
        match kills.get_mut(&npc) {
            Some(kept) => {
                let name = kept.name.take().or_else(|| kill.name.clone());
                if prefer(kept.count as usize, kill.count as usize) {
                    *kept = kill;
                }
                kept.name = kept.name.take().or(name);
            }
            None => {
                kills.insert(npc, kill);
            }
        }
    }

    let mut skills = stored.skills;
    for (id, skill) in incoming.skills {
        match skills.get_mut(&id) {
            Some(kept) => {
                kept.from = kept.from.min(skill.from);
                kept.to = kept.to.max(skill.to);
                kept.name = kept.name.take().or(skill.name);
                kept.category = kept.category.take().or(skill.category);
            }
            None => {
                skills.insert(id, skill);
            }
        }
    }

    let items = |mut kept: BTreeMap<i64, ItemCount>, incoming: BTreeMap<i64, ItemCount>| {
        for (id, item) in incoming {
            match kept.get_mut(&id) {
                Some(k) => {
                    k.count = k.count.max(item.count);
                    k.name = k.name.take().or(item.name);
                    k.via = k.via.take().or(item.via);
                }
                None => {
                    kept.insert(id, item);
                }
            }
        }
        kept
    };

    let digest = match (stored.digest, incoming.digest) {
        (Some(s), Some(i)) => Some(
            if fullness(&i) > fullness(&s) || (fullness(&i) == fullness(&s) && incoming_newer) {
                i
            } else {
                s
            },
        ),
        (s, i) => i.or(s),
    };

    Day {
        events,
        kills,
        anon_kills: stored.anon_kills.max(incoming.anon_kills),
        skills,
        crafted: items(stored.crafted, incoming.crafted),
        gathered: items(stored.gathered, incoming.gathered),
        digest,
        // The longer event list is kept, and a whole day's is longer than a compacted one's.
        compacted: stored.compacted && incoming.compacted,
    }
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
/// The shape archived rows are written in. A row with a higher format was written by a newer
/// app: it is never read as if it were this one.
pub const ARCHIVE_FORMAT: i64 = 1;

/// Reads an archived row, or `None` when it cannot be read: damaged, or written by a newer app.
fn read_row<T: serde::de::DeserializeOwned>(json: &str, format: i64) -> Option<T> {
    if format > ARCHIVE_FORMAT {
        return None;
    }
    serde_json::from_str(json).ok()
}

/// Keeps an unreadable row aside before anything replaces it, so it can still be recovered.
fn set_aside(
    tx: &rusqlite::Transaction,
    character_id: i64,
    day: &str,
    json: &str,
    format: i64,
) -> Result<(), ArchiveError> {
    tracing::warn!("archive: day {day} of character {character_id} could not be read; set aside");
    tx.execute(
        "INSERT INTO archive_quarantine (character_id, day, data_json, format, set_aside)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            character_id,
            day,
            json,
            format,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

pub fn merge_snapshot(
    db: &Database,
    character_id: i64,
    journal: &Journal,
    source_mtime: i64,
) -> Result<(), ArchiveError> {
    let conn = db.conn();
    let tx = conn.unchecked_transaction()?;

    for (date, incoming) in &journal.days {
        let stored: Option<(String, i64, i64)> = tx
            .query_row(
                "SELECT data_json, source_mtime, format FROM archive_days
                 WHERE character_id = ?1 AND day = ?2",
                params![character_id, date],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (merged, mtime) = match stored {
            None => (incoming.clone(), source_mtime),
            Some((json, stored_mtime, format)) => match read_row::<Day>(&json, format) {
                Some(stored) => {
                    let newer = source_mtime >= stored_mtime;
                    (
                        merge_day(stored, incoming.clone(), newer),
                        stored_mtime.max(source_mtime),
                    )
                }
                None => {
                    set_aside(&tx, character_id, date, &json, format)?;
                    (incoming.clone(), source_mtime)
                }
            },
        };
        tx.execute(
            "INSERT INTO archive_days (character_id, day, data_json, source_mtime, format)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (character_id, day) DO UPDATE SET
                 data_json = excluded.data_json, source_mtime = excluded.source_mtime,
                 format = excluded.format",
            params![
                character_id,
                date,
                serde_json::to_string(&merged)?,
                mtime,
                ARCHIVE_FORMAT
            ],
        )?;
    }

    let stored: Option<(String, i64, i64)> = tx
        .query_row(
            "SELECT data_json, source_mtime, format FROM archive_journals WHERE character_id = ?1",
            [character_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (mut record, stored_mtime) = match stored {
        Some((json, mtime, format)) => match read_row::<JournalRecord>(&json, format) {
            Some(record) => (record, mtime),
            None => {
                set_aside(&tx, character_id, "", &json, format)?;
                (JournalRecord::default(), i64::MIN)
            }
        },
        None => (JournalRecord::default(), i64::MIN),
    };
    let newer = source_mtime >= stored_mtime;
    if newer {
        record.schema = journal.schema;
        record.character = journal.character.clone();
    }
    // All-time counts only grow: an old file read late, or a Forget in the game, never lowers them.
    let stats = &journal.stats;
    record.stats.boots = record.stats.boots.max(stats.boots);
    record.stats.murlocs_slain = record.stats.murlocs_slain.max(stats.murlocs_slain);
    record.stats.deaths_by_murloc = record.stats.deaths_by_murloc.max(stats.deaths_by_murloc);
    record.stats.drownings = record.stats.drownings.max(stats.drownings);
    // The latest login and the larger error count, whatever order the files arrive in.
    if let Some(addon) = &journal.addon
        && record
            .addon
            .as_ref()
            .is_none_or(|kept| kept.login <= addon.login)
    {
        record.addon = Some(addon.clone());
    }
    if let Some(errors) = &journal.errors
        && record
            .errors
            .as_ref()
            .is_none_or(|kept| kept.count <= errors.count)
    {
        record.errors = Some(errors.clone());
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
        let kept = record.companions.entry(who.clone()).or_default();
        kept.days = kept.days.max(companion.days);
        kept.first = min_some(kept.first.take(), companion.first.clone());
        kept.last = kept.last.take().max(companion.last.clone());
    }
    record.discovered.extend(journal.discovered.iter().cloned());
    tx.execute(
        "INSERT INTO archive_journals (character_id, data_json, source_mtime, format)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (character_id) DO UPDATE SET
             data_json = excluded.data_json, source_mtime = excluded.source_mtime,
             format = excluded.format",
        params![
            character_id,
            serde_json::to_string(&record)?,
            stored_mtime.max(source_mtime),
            ARCHIVE_FORMAT
        ],
    )?;
    tx.commit()?;
    Ok(())
}

/// A character's archive, and the days that could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub journal: Journal,
    /// Days whose rows are damaged or were written by a newer app; `""` for the journal's own
    /// record. They are left in place, and set aside before a snapshot replaces them.
    pub unreadable: Vec<String>,
}

/// A character's archived journal: every day the app has seen, merged. A row that cannot be read
/// is skipped, so one damaged day never hides the others.
pub fn load(db: &Database, character_id: i64) -> Result<Option<Journal>, ArchiveError> {
    Ok(load_checked(db, character_id)?.map(|loaded| loaded.journal))
}

/// What the addon last noted in a character's journal: its login heartbeat and the errors it
/// caught. Reads the journal's record only, not its days.
pub fn addon_notes(
    db: &Database,
    character_id: i64,
) -> Result<(Option<AddonHeartbeat>, Option<AddonErrors>), ArchiveError> {
    let record: Option<(String, i64)> = db
        .conn()
        .query_row(
            "SELECT data_json, format FROM archive_journals WHERE character_id = ?1",
            [character_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(record
        .and_then(|(json, format)| read_row::<JournalRecord>(&json, format))
        .map(|record| (record.addon, record.errors))
        .unwrap_or_default())
}

/// The archived journal, with the days that could not be read.
pub fn load_checked(db: &Database, character_id: i64) -> Result<Option<Loaded>, ArchiveError> {
    let conn = db.conn();
    let record: Option<(String, i64)> = conn
        .query_row(
            "SELECT data_json, format FROM archive_journals WHERE character_id = ?1",
            [character_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((record, format)) = record else {
        return Ok(None);
    };
    let mut unreadable = Vec::new();
    let record: JournalRecord = read_row(&record, format).unwrap_or_else(|| {
        unreadable.push(String::new());
        JournalRecord::default()
    });
    let mut stmt =
        conn.prepare("SELECT day, data_json, format FROM archive_days WHERE character_id = ?1")?;
    let rows = stmt.query_map([character_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut days = BTreeMap::new();
    for row in rows {
        let (date, json, format) = row?;
        match read_row::<Day>(&json, format) {
            Some(day) => {
                days.insert(date, day);
            }
            None => {
                tracing::warn!("archive: day {date} of character {character_id} could not be read");
                unreadable.push(date);
            }
        }
    }
    unreadable.sort();
    Ok(Some(Loaded {
        journal: Journal {
            schema: record.schema,
            character: record.character,
            days,
            quests: record.quests,
            companions: record.companions,
            discovered: record.discovered,
            stats: record.stats,
            addon: record.addon,
            errors: record.errors,
        },
        unreadable,
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
