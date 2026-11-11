//! The app's own SQLite database, in the app data folder: settings and the characters it knows.
//! Never anything under the WoW folder.
//!
//! The schema evolves through an ordered list of migrations recorded in `PRAGMA user_version`,
//! one transaction each, so a crash mid-upgrade leaves the previous version intact.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::discovery::CharacterSource;
use crate::narrators::DEFAULT_NARRATOR;
use crate::prompt::Meddling;
use crate::schema::Character;
use crate::usage::{Audience, DataUse};

/// The database file name inside the app data folder.
pub const DATABASE_FILE: &str = "mrglglgl.db";

/// How long backups of the raw journal files are kept. The archive keeps every day it has seen
/// whatever this is set to; backups are only a safety net of the files as the game wrote them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum BackupRetention {
    Days30,
    #[default]
    Days90,
    Year,
    Forever,
}

impl BackupRetention {
    /// The period in days, or `None` for forever.
    pub fn days(self) -> Option<i64> {
        match self {
            BackupRetention::Days30 => Some(30),
            BackupRetention::Days90 => Some(90),
            BackupRetention::Year => Some(365),
            BackupRetention::Forever => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            BackupRetention::Days30 => "days30",
            BackupRetention::Days90 => "days90",
            BackupRetention::Year => "year",
            BackupRetention::Forever => "forever",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        [Self::Days30, Self::Days90, Self::Year, Self::Forever]
            .into_iter()
            .find(|r| r.key() == key)
    }
}

/// Migrations in order; entry `n` upgrades the database from version `n` to `n + 1`.
const MIGRATIONS: &[&str] = &[
    // Settings are key/value so adding one needs no migration. A character is identified by
    // where its journal lives: the folder names are opaque (on WoW: Forever the realm folder is a
    // number unrelated to the realm), so names come from the journal and are display only.
    "CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE characters (
        id INTEGER PRIMARY KEY,
        flavor TEXT NOT NULL,
        account TEXT NOT NULL,
        realm_folder TEXT NOT NULL,
        character_folder TEXT NOT NULL,
        journal_path TEXT NOT NULL,
        name TEXT,
        surname TEXT,
        realm TEXT,
        race TEXT,
        class TEXT,
        faction TEXT,
        locale TEXT,
        last_synced INTEGER,
        UNIQUE (flavor, account, realm_folder, character_folder)
    );",
    // The archive: everything the app has seen of each character, so in-game retention can stay
    // short and a character survives its game files. A day is stored whole as JSON; it is only
    // ever read whole. `status` records why a character's latest journal could not be read.
    "ALTER TABLE characters ADD COLUMN status TEXT;
    CREATE TABLE archive_days (
        character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
        day TEXT NOT NULL,
        data_json TEXT NOT NULL,
        source_mtime INTEGER NOT NULL,
        PRIMARY KEY (character_id, day)
    );
    CREATE TABLE archive_journals (
        character_id INTEGER PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
        data_json TEXT NOT NULL,
        source_mtime INTEGER NOT NULL
    );
    CREATE TABLE milestones (
        character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
        id TEXT NOT NULL,
        day TEXT NOT NULL,
        PRIMARY KEY (character_id, id)
    );",
    // Backups are kept by age instead of by count. The untouched default (30 copies) becomes the
    // new default of 90 days; a player who kept fewer copies gets the shortest period, one who
    // kept more gets a year.
    "INSERT OR REPLACE INTO settings (key, value)
        SELECT 'backup_retention',
            CASE
                WHEN CAST(value AS INTEGER) = 30 THEN 'days90'
                WHEN CAST(value AS INTEGER) < 30 THEN 'days30'
                ELSE 'year'
            END
        FROM settings WHERE key = 'backup_count';
    DELETE FROM settings WHERE key = 'backup_count';",
    // What each character's story carries from one entry to the next.
    "CREATE TABLE continuity (
        character_id INTEGER PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
        sheet_json TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    );",
    // Written entries, every version kept, and the queue of days and weeks waiting to be
    // written. `check_status` records the fact check; `not_before` holds a retry back; `priority`
    // puts a day the player asked for ahead of the backlog; `narrator` is a one-off choice.
    "CREATE TABLE stories (
        id INTEGER PRIMARY KEY,
        character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
        kind TEXT NOT NULL CHECK (kind IN ('day', 'week')),
        start_day TEXT NOT NULL,
        end_day TEXT NOT NULL,
        narrator TEXT NOT NULL,
        language TEXT NOT NULL,
        provider TEXT NOT NULL,
        model TEXT NOT NULL,
        prompt_hash TEXT NOT NULL,
        text TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        check_status TEXT NOT NULL DEFAULT 'passed',
        check_reason TEXT
    );
    CREATE INDEX stories_by_slot ON stories (character_id, kind, start_day);
    CREATE TABLE write_queue (
        character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
        kind TEXT NOT NULL CHECK (kind IN ('day', 'week')),
        start_day TEXT NOT NULL,
        state TEXT NOT NULL CHECK (state IN ('pending', 'running', 'failed')),
        attempts INTEGER NOT NULL DEFAULT 0,
        last_error TEXT,
        not_before INTEGER NOT NULL DEFAULT 0,
        priority INTEGER NOT NULL DEFAULT 0,
        narrator TEXT,
        PRIMARY KEY (character_id, kind, start_day)
    );",
    // Draft review: a story is accepted (in the journal), a draft waiting for the player, or a
    // draft the player threw away. Stories written before review existed were all accepted. A
    // job knows whether its result is a draft and carries the player's note for a rewrite.
    "ALTER TABLE stories ADD COLUMN status TEXT NOT NULL DEFAULT 'accepted'
        CHECK (status IN ('accepted', 'draft', 'discarded'));
    ALTER TABLE stories ADD COLUMN note TEXT;
    ALTER TABLE write_queue ADD COLUMN draft INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE write_queue ADD COLUMN note TEXT;",
    // A day told so far, while it is still going: shown at once, but it never stands in for the
    // full entry, which is still written once the day is over.
    "ALTER TABLE stories ADD COLUMN partial INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE write_queue ADD COLUMN partial INTEGER NOT NULL DEFAULT 0;",
    // Archived rows carry the format they were written in, so a newer app's rows are never
    // misread. A row that cannot be read is set aside here before anything replaces it.
    "ALTER TABLE archive_days ADD COLUMN format INTEGER NOT NULL DEFAULT 1;
    ALTER TABLE archive_journals ADD COLUMN format INTEGER NOT NULL DEFAULT 1;
    CREATE TABLE archive_quarantine (
        character_id INTEGER NOT NULL,
        day TEXT NOT NULL,
        data_json TEXT NOT NULL,
        format INTEGER NOT NULL,
        set_aside INTEGER NOT NULL
    );",
    // Recorded categories the player set aside for a character, as JSON (see `DataUse`); none
    // means the app uses everything. On the character's row so it goes with it.
    "ALTER TABLE characters ADD COLUMN unused_data TEXT;",
    // How a character's entries are written when it differs from Settings, as JSON (see
    // `CharacterWriting`); none means Settings decide. On the character's row so it goes with it.
    "ALTER TABLE characters ADD COLUMN writing TEXT;",
    // Whether the character is in the app: when not, its journal is no longer read, its
    // entries wait unwritten, and it is out of sight. Its history stays.
    "ALTER TABLE characters ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1;",
    // Entries and drafts with no text, which a model that answered nothing used to leave behind
    // as if the day were written. Writing now fails instead, so the day waits to be written.
    "DELETE FROM stories WHERE trim(text) = '';",
    // A set-aside entry's reason was stored as an English sentence; it is now only what the entry
    // mentions that the notes do not, so the app can say it in the player's language.
    "UPDATE stories SET check_reason = substr(check_reason, 10, length(check_reason) - 9 - 23)
     WHERE check_reason LIKE 'mentions % not in the day''s notes';",
];

/// The schema version this build creates and understands.
pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not create the data folder {path}: {source}")]
    DataFolder {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "the database was created by a newer version of the app (version {found}, this one knows up to {known}): update the app"
    )]
    NewerDatabase { found: i64, known: i64 },
}

/// Who writes the entries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum ProviderChoice {
    /// The bundled model, on this machine.
    #[default]
    Local,
    /// An OpenAI-compatible endpoint.
    Endpoint,
}

/// How entries are written. Stored as one JSON value, so a new field needs no migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", default)]
pub struct WritingSettings {
    pub narrator: String,
    pub meddling: Meddling,
    /// The prose language; `None` follows the app's language.
    pub prose_language: Option<String>,
    pub provider: ProviderChoice,
    /// The catalogue model to write with; `None` uses the recommended one.
    pub model_id: Option<String>,
    pub endpoint_url: Option<String>,
    pub endpoint_model: Option<String>,
    /// Send companions to an endpoint under stand-in names.
    pub pseudonyms: bool,
    /// Write in the background even while the game is running. Off by default: inference uses
    /// the CPU the game needs.
    pub write_while_playing: bool,
    /// Write in the background while the computer runs on battery. Off by default: a minute of
    /// full processor per entry drains a laptop.
    pub write_on_battery: bool,
    /// The background queue is paused.
    pub paused: bool,
    /// Entries written in the background wait as drafts for the player to accept. Off by
    /// default: opening the app finds the finished days already written.
    pub review: bool,
    /// "Copy entry" adds a one-line "via Mrglglgl" footer.
    pub copy_footer: bool,
    /// Each finished week is also told as one chapter, from its days' entries. Off by default:
    /// the days are the journal, and the chapter is a recap the player asks for. Off, no chapter
    /// is queued or listed, and chapters already written stay.
    pub weekly: bool,
    /// How the player likes the prose written, sent with every entry and chapter ("no em
    /// dashes", "British spelling"). Style only: never taken as a fact of the day.
    pub preferences: String,
    /// How many times the model is asked again when an entry mentions something the day's notes
    /// do not, before the entry is set aside for the player to decide. Each try costs a full
    /// generation, so the default asks once.
    pub check_retries: u32,
}

/// What one character's entries are written with where it differs from Settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", default)]
pub struct CharacterWriting {
    /// The narrator for this character; `None` uses the one in Settings.
    pub narrator: Option<String>,
    /// How often the murloc writes in the margin; `None` follows Settings.
    pub meddling: Option<Meddling>,
    /// Added after the writing preferences in Settings, for this character alone.
    pub preferences: String,
}

impl WritingSettings {
    /// The settings one character's entries are written with: its own narrator and margin notes
    /// where it has them, and its preferences after the ones every character shares.
    pub fn for_character(&self, own: &CharacterWriting) -> WritingSettings {
        let mut settings = self.clone();
        if let Some(narrator) = &own.narrator {
            settings.narrator = narrator.clone();
        }
        if let Some(meddling) = own.meddling {
            settings.meddling = meddling;
        }
        let added = own.preferences.trim();
        if !added.is_empty() {
            settings.preferences = match settings.preferences.trim() {
                "" => added.to_string(),
                general => format!("{general}\n{added}"),
            };
        }
        settings
    }
}

/// Tries after the first when the fact check fails, unless the player chose otherwise.
pub const DEFAULT_CHECK_RETRIES: u32 = 1;
/// The most tries after the first the player may ask for: each is a full generation.
pub const MAX_CHECK_RETRIES: u32 = 5;

impl Default for WritingSettings {
    fn default() -> Self {
        Self {
            narrator: DEFAULT_NARRATOR.to_string(),
            meddling: Meddling::default(),
            prose_language: None,
            provider: ProviderChoice::default(),
            model_id: None,
            endpoint_url: None,
            endpoint_model: None,
            pseudonyms: false,
            write_while_playing: false,
            write_on_battery: false,
            paused: false,
            review: false,
            copy_footer: true,
            weekly: false,
            preferences: String::new(),
            check_retries: DEFAULT_CHECK_RETRIES,
        }
    }
}

/// The journal website: where it is written, how it looks, and whose journals it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", default)]
pub struct SiteSettings {
    /// The folder is rebuilt while the app is open.
    pub enabled: bool,
    pub folder: Option<PathBuf>,
    /// A theme of the player's own; its files win over the default theme's.
    pub theme_folder: Option<PathBuf>,
    pub title: String,
    /// Where the site will live, for the feeds' full links.
    pub base_url: Option<String>,
    /// The characters on the site, by id.
    pub characters: Vec<i64>,
    /// Other players appear under stand-in names.
    pub stand_in_names: bool,
    /// Realm and account names are left out.
    pub hide_realm: bool,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            folder: None,
            theme_folder: None,
            title: String::new(),
            base_url: None,
            characters: Vec::new(),
            stand_in_names: true,
            hide_realm: true,
        }
    }
}

/// The settings the player chooses in the app.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// A WoW folder chosen by the player; wins over every discovered install.
    pub wow_root_override: Option<PathBuf>,
    /// How long backups of each character's journal file are kept in the app data folder.
    pub backup_retention: BackupRetention,
    /// The UI language; `None` follows the operating system.
    pub ui_language: Option<String>,
    pub writing: WritingSettings,
    #[serde(default)]
    pub site: SiteSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct StoredCharacter {
    pub id: i64,
    pub flavor: String,
    pub account: String,
    pub realm_folder: String,
    pub character_folder: String,
    pub journal_path: PathBuf,
    pub name: Option<String>,
    pub surname: Option<String>,
    pub realm: Option<String>,
    pub race: Option<String>,
    pub class: Option<String>,
    pub faction: Option<String>,
    pub locale: Option<String>,
    /// Unix time the journal file was last read.
    pub last_synced: Option<i64>,
    /// Why the latest journal could not be read (`"older"`, `"newer"`, `"unreadable"`), if so.
    pub status: Option<String>,
    /// In the app; when not, its journal is not read and nothing is written for it.
    pub enabled: bool,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens the database in `data_dir`, creating the folder and the database as needed, and
    /// brings it to the current version.
    pub fn open(data_dir: &Path) -> Result<Self, DbError> {
        std::fs::create_dir_all(data_dir).map_err(|source| DbError::DataFolder {
            path: data_dir.to_path_buf(),
            source,
        })?;
        let conn = Connection::open(data_dir.join(DATABASE_FILE))?;
        // WAL keeps the UI reading while a background import writes.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::prepare(conn)
    }

    /// Copies the whole database into `<data_dir>/snapshots/` when the newest copy there is a
    /// week old or more, keeping the four newest. The archive is the only copy of days the game
    /// has since compacted, so it should not live in one file. Returns the new copy, if any.
    pub fn snapshot_if_due(&self, data_dir: &Path, now: i64) -> Result<Option<PathBuf>, DbError> {
        const KEEP: usize = 4;
        const EVERY_DAYS: i64 = 7;
        let folder = data_dir.join("snapshots");
        let io = |source| DbError::DataFolder {
            path: folder.clone(),
            source,
        };
        std::fs::create_dir_all(&folder).map_err(io)?;
        let mut taken: Vec<(chrono::NaiveDate, PathBuf)> = std::fs::read_dir(&folder)
            .map_err(io)?
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let date = name.strip_prefix("mrglglgl-")?.strip_suffix(".db")?;
                Some((
                    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?,
                    e.path(),
                ))
            })
            .collect();
        taken.sort();
        let today = chrono::DateTime::from_timestamp(now, 0)
            .unwrap_or_default()
            .date_naive();
        if taken
            .last()
            .is_some_and(|(date, _)| (today - *date).num_days() < EVERY_DAYS)
        {
            return Ok(None);
        }
        let path = folder.join(format!("mrglglgl-{}.db", today.format("%Y-%m-%d")));
        self.conn
            .execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
        taken.push((today, path.clone()));
        let excess = taken.len().saturating_sub(KEEP);
        for (_, old) in taken.into_iter().take(excess) {
            std::fs::remove_file(&old).map_err(io)?;
        }
        Ok(Some(path))
    }

    /// An in-memory database, for tests and previews.
    pub fn open_in_memory() -> Result<Self, DbError> {
        Self::prepare(Connection::open_in_memory()?)
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    fn prepare(mut conn: Connection) -> Result<Self, DbError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn version(&self) -> Result<i64, DbError> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    fn setting(&self, key: &str) -> Result<Option<String>, DbError> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    fn set_setting(&self, key: &str, value: Option<&str>) -> Result<(), DbError> {
        match value {
            Some(value) => self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?,
            None => self
                .conn
                .execute("DELETE FROM settings WHERE key = ?1", [key])?,
        };
        Ok(())
    }

    /// The player's settings, with defaults for anything never set.
    pub fn settings(&self) -> Result<Settings, DbError> {
        Ok(self.settings_checked()?.0)
    }

    /// A settings group stored as JSON; one that cannot be read falls back to its defaults, is
    /// logged, and its stored text is copied aside under `<key>.unreadable` (once), so the next
    /// save does not destroy the only copy.
    fn json_setting<T: serde::de::DeserializeOwned + Default>(
        &self,
        key: &str,
        unreadable: &mut Vec<String>,
    ) -> Result<T, DbError> {
        let Some(json) = self.setting(key)? else {
            return Ok(T::default());
        };
        match serde_json::from_str(&json) {
            Ok(value) => Ok(value),
            Err(e) => {
                tracing::warn!("settings: {key} could not be read ({e}); using its defaults");
                let aside = format!("{key}.unreadable");
                if self.setting(&aside)?.is_none() {
                    self.set_setting(&aside, Some(&json))?;
                }
                unreadable.push(key.to_string());
                Ok(T::default())
            }
        }
    }

    /// The player's settings, and the groups that could not be read and are at their defaults.
    pub fn settings_checked(&self) -> Result<(Settings, Vec<String>), DbError> {
        let defaults = Settings::default();
        let mut unreadable = Vec::new();
        let backup_retention = match self.setting("backup_retention")? {
            None => defaults.backup_retention,
            Some(v) => BackupRetention::from_key(&v).unwrap_or_else(|| {
                tracing::warn!(
                    "settings: backup_retention {v:?} is not a choice; using the default"
                );
                unreadable.push("backup_retention".to_string());
                defaults.backup_retention
            }),
        };
        let settings = Settings {
            wow_root_override: self.setting("wow_root_override")?.map(PathBuf::from),
            backup_retention,
            ui_language: self.setting("ui_language")?,
            writing: self.json_setting("writing", &mut unreadable)?,
            site: self.json_setting("site", &mut unreadable)?,
        };
        Ok((settings, unreadable))
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction()?;
        let root = settings
            .wow_root_override
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());
        self.set_setting("wow_root_override", root.as_deref())?;
        self.set_setting("backup_retention", Some(settings.backup_retention.key()))?;
        self.set_setting("ui_language", settings.ui_language.as_deref())?;
        let writing = serde_json::to_string(&settings.writing).expect("settings serialise");
        self.set_setting("writing", Some(&writing))?;
        let site = serde_json::to_string(&settings.site).expect("settings serialise");
        self.set_setting("site", Some(&site))?;
        tx.commit()?;
        Ok(())
    }

    /// Whether the addon's test versions are offered as well as its releases.
    pub fn addon_prereleases(&self) -> Result<bool, DbError> {
        Ok(self.setting("addon_prereleases")?.as_deref() == Some("1"))
    }

    pub fn set_addon_prereleases(&self, on: bool) -> Result<(), DbError> {
        self.set_setting("addon_prereleases", on.then_some("1"))
    }

    /// Records a character found on disk, with what its journal says about it when it was read.
    /// Returns the character's id, which stays the same for the same journal location.
    pub fn upsert_character(
        &self,
        source: &CharacterSource,
        character: Option<&Character>,
        last_synced: Option<i64>,
    ) -> Result<i64, DbError> {
        let c = character.cloned().unwrap_or_default();
        Ok(self.conn.query_row(
            "INSERT INTO characters (flavor, account, realm_folder, character_folder,
                 journal_path, name, surname, realm, race, class, faction, locale, last_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT (flavor, account, realm_folder, character_folder) DO UPDATE SET
                 journal_path = excluded.journal_path,
                 name = coalesce(excluded.name, name),
                 surname = coalesce(excluded.surname, surname),
                 realm = coalesce(excluded.realm, realm),
                 race = coalesce(excluded.race, race),
                 class = coalesce(excluded.class, class),
                 faction = coalesce(excluded.faction, faction),
                 locale = coalesce(excluded.locale, locale),
                 last_synced = coalesce(excluded.last_synced, last_synced)
             RETURNING id",
            params![
                source.flavor,
                source.account,
                source.realm_folder,
                source.character_folder,
                source.journal_path.to_string_lossy(),
                c.name,
                c.surname,
                c.realm,
                c.race,
                c.class,
                c.faction,
                c.locale,
                last_synced,
            ],
            |row| row.get(0),
        )?)
    }

    /// Every known character, grouped the way the sidebar shows them.
    pub fn characters(&self) -> Result<Vec<StoredCharacter>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, flavor, account, realm_folder, character_folder, journal_path, name,
                    surname, realm, race, class, faction, locale, last_synced, status, enabled
             FROM characters
             ORDER BY flavor, account, coalesce(realm, realm_folder), coalesce(name, character_folder)",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StoredCharacter {
                id: row.get(0)?,
                flavor: row.get(1)?,
                account: row.get(2)?,
                realm_folder: row.get(3)?,
                character_folder: row.get(4)?,
                journal_path: PathBuf::from(row.get::<_, String>(5)?),
                name: row.get(6)?,
                surname: row.get(7)?,
                realm: row.get(8)?,
                race: row.get(9)?,
                class: row.get(10)?,
                faction: row.get(11)?,
                locale: row.get(12)?,
                last_synced: row.get(13)?,
                status: row.get(14)?,
                enabled: row.get(15)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

impl Database {
    /// Records why a character's latest journal could not be read, or clears it.
    pub fn set_character_status(&self, id: i64, status: Option<&str>) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE characters SET status = ?2 WHERE id = ?1",
            params![id, status],
        )?;
        Ok(())
    }

    /// Puts a character in the app or takes it out. Out of the app its history is kept, but its
    /// journal is not read and its entries are not written until it is back.
    pub fn set_character_enabled(&self, id: i64, enabled: bool) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE characters SET enabled = ?2 WHERE id = ?1",
            params![id, enabled],
        )?;
        Ok(())
    }

    /// Whether the journal at `source` belongs to a character the player took out of the app.
    /// A character never seen before is in the app.
    pub fn is_disabled(&self, source: &CharacterSource) -> Result<bool, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT enabled = 0 FROM characters
                 WHERE flavor = ?1 AND account = ?2 AND realm_folder = ?3 AND character_folder = ?4",
                params![
                    source.flavor,
                    source.account,
                    source.realm_folder,
                    source.character_folder
                ],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(false))
    }

    /// Forgets a character and everything archived for it. Backups on disk are the archive's to
    /// remove.
    pub fn delete_character(&self, id: i64) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM characters WHERE id = ?1", [id])?;
        Ok(())
    }

    /// What the player set aside for a character, for each audience; nothing when it uses
    /// everything. A choice that cannot be read sets every category aside from both, so nothing
    /// the player hid is shared, and says so in the log; saving the choice again repairs it.
    pub fn data_use(&self, id: i64) -> Result<DataUse, DbError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT unused_data FROM characters WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let Some(json) = json else {
            return Ok(DataUse::default());
        };
        Ok(serde_json::from_str(&json).unwrap_or_else(|e| {
            tracing::warn!("character {id}: the data it uses could not be read ({e}); using none");
            let all: BTreeSet<String> = crate::usage::CATEGORIES
                .iter()
                .map(|c| c.to_string())
                .collect();
            DataUse {
                shared: all.clone(),
                view: all,
            }
        }))
    }

    /// The categories set aside from one audience.
    pub fn hidden(&self, id: i64, audience: Audience) -> Result<BTreeSet<String>, DbError> {
        let choice = self.data_use(id)?;
        Ok(match audience {
            Audience::Shared => choice.shared,
            Audience::View => choice.view,
        })
    }

    pub fn set_data_use(&self, id: i64, choice: &DataUse) -> Result<(), DbError> {
        let json = (!choice.shared.is_empty() || !choice.view.is_empty())
            .then(|| serde_json::to_string(choice).expect("sets of strings serialize"));
        self.conn.execute(
            "UPDATE characters SET unused_data = ?2 WHERE id = ?1",
            params![id, json],
        )?;
        Ok(())
    }

    /// How this character's entries are written where it differs from Settings. A choice that
    /// cannot be read falls back to Settings, and says so in the log.
    pub fn character_writing(&self, id: i64) -> Result<CharacterWriting, DbError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT writing FROM characters WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(json
            .and_then(|json| {
                serde_json::from_str(&json)
                    .map_err(|e| {
                        tracing::warn!(
                            "character {id}: its writing choices could not be read ({e})"
                        )
                    })
                    .ok()
            })
            .unwrap_or_default())
    }

    pub fn set_character_writing(&self, id: i64, choice: &CharacterWriting) -> Result<(), DbError> {
        let json = (choice != &CharacterWriting::default())
            .then(|| serde_json::to_string(choice).expect("plain fields serialize"));
        self.conn.execute(
            "UPDATE characters SET writing = ?2 WHERE id = ?1",
            params![id, json],
        )?;
        Ok(())
    }

    pub fn character(&self, id: i64) -> Result<Option<StoredCharacter>, DbError> {
        Ok(self.characters()?.into_iter().find(|c| c.id == id))
    }
}

fn migrate(conn: &mut Connection) -> Result<(), DbError> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > CURRENT_VERSION {
        return Err(DbError::NewerDatabase {
            found: version,
            known: CURRENT_VERSION,
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}
