//! The app's own SQLite database, in the app data folder: settings and the characters it knows.
//! Never anything under the WoW folder.
//!
//! The schema evolves through an ordered list of migrations recorded in `PRAGMA user_version`,
//! one transaction each, so a crash mid-upgrade leaves the previous version intact.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::discovery::CharacterSource;
use crate::schema::Character;

/// The database file name inside the app data folder.
pub const DATABASE_FILE: &str = "mrglglgl.db";

/// Backups kept per character when the player has not chosen a number.
pub const DEFAULT_BACKUP_COUNT: u32 = 30;

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

/// The settings the player chooses in the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// A WoW folder chosen by the player; wins over every discovered install.
    pub wow_root_override: Option<PathBuf>,
    /// Backups of each character's journal file kept in the app data folder.
    pub backup_count: u32,
    /// The UI language; `None` follows the operating system.
    pub ui_language: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wow_root_override: None,
            backup_count: DEFAULT_BACKUP_COUNT,
            ui_language: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
        let defaults = Settings::default();
        Ok(Settings {
            wow_root_override: self.setting("wow_root_override")?.map(PathBuf::from),
            backup_count: self
                .setting("backup_count")?
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.backup_count),
            ui_language: self.setting("ui_language")?,
        })
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction()?;
        let root = settings
            .wow_root_override
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());
        self.set_setting("wow_root_override", root.as_deref())?;
        self.set_setting("backup_count", Some(&settings.backup_count.to_string()))?;
        self.set_setting("ui_language", settings.ui_language.as_deref())?;
        tx.commit()?;
        Ok(())
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
                    surname, realm, race, class, faction, locale, last_synced, status
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

    /// Forgets a character and everything archived for it. Backups on disk are the archive's to
    /// remove.
    pub fn delete_character(&self, id: i64) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM characters WHERE id = ?1", [id])?;
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
