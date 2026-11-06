use std::path::PathBuf;

use mrglglgl_core::db::{
    CURRENT_VERSION, DATABASE_FILE, DEFAULT_BACKUP_COUNT, Database, DbError, Settings,
};
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::schema::Character;

fn source(character_folder: &str) -> CharacterSource {
    CharacterSource {
        flavor: "_forever_".into(),
        account: "ACCOUNT1".into(),
        realm_folder: "93".into(),
        character_folder: character_folder.into(),
        journal_path: PathBuf::from(format!(
            "/wow/_forever_/WTF/Account/ACCOUNT1/93/{character_folder}/SavedVariables/Mrglglgl.lua"
        )),
    }
}

fn wren() -> Character {
    Character {
        name: Some("Wren".into()),
        surname: Some("Hollowbrook".into()),
        realm: Some("Test Realm".into()),
        realm_id: Some(1),
        race: Some("Tauren".into()),
        class: Some("HUNTER".into()),
        faction: Some("Horde".into()),
        locale: Some("enUS".into()),
    }
}

#[test]
fn creates_the_database_at_the_current_version_in_an_empty_data_folder() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("not/yet/there");
    let db = Database::open(&data).unwrap();
    assert!(data.join(DATABASE_FILE).is_file());
    assert_eq!(db.version().unwrap(), CURRENT_VERSION);
}

#[test]
fn opening_again_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.upsert_character(&source("Wren-Hollowbrook"), Some(&wren()), Some(100))
            .unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.version().unwrap(), CURRENT_VERSION);
    assert_eq!(db.characters().unwrap().len(), 1);
}

#[test]
fn refuses_a_database_from_a_newer_app() {
    let dir = tempfile::tempdir().unwrap();
    drop(Database::open(dir.path()).unwrap());
    let conn = rusqlite::Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
    conn.pragma_update(None, "user_version", CURRENT_VERSION + 1)
        .unwrap();
    drop(conn);
    match Database::open(dir.path()) {
        Err(DbError::NewerDatabase { found, known }) => {
            assert_eq!((found, known), (CURRENT_VERSION + 1, CURRENT_VERSION));
        }
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("opened a database from a newer app"),
    }
}

#[test]
fn settings_default_until_saved_and_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.settings().unwrap(), Settings::default());
        assert_eq!(db.settings().unwrap().backup_count, DEFAULT_BACKUP_COUNT);
        db.save_settings(&Settings {
            wow_root_override: Some(PathBuf::from("/Games/World of Warcraft")),
            backup_count: 10,
            ui_language: Some("fr".into()),
        })
        .unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    let settings = db.settings().unwrap();
    assert_eq!(
        settings.wow_root_override,
        Some(PathBuf::from("/Games/World of Warcraft"))
    );
    assert_eq!(settings.backup_count, 10);
    assert_eq!(settings.ui_language.as_deref(), Some("fr"));

    db.save_settings(&Settings::default()).unwrap();
    assert_eq!(
        db.settings().unwrap(),
        Settings::default(),
        "clearing works too"
    );
}

#[test]
fn keeps_one_row_per_journal_location_and_keeps_known_names() {
    let db = Database::open_in_memory().unwrap();
    let first = db
        .upsert_character(&source("Wren-Hollowbrook"), Some(&wren()), Some(100))
        .unwrap();
    // Seen again on disk before its journal was read: the id and the name stay.
    let again = db
        .upsert_character(&source("Wren-Hollowbrook"), None, None)
        .unwrap();
    assert_eq!(first, again);
    let other = db
        .upsert_character(&source("Tamsin-Reedwater"), None, None)
        .unwrap();
    assert_ne!(first, other);

    let characters = db.characters().unwrap();
    assert_eq!(characters.len(), 2);
    let stored = characters.iter().find(|c| c.id == first).unwrap();
    assert_eq!(stored.name.as_deref(), Some("Wren"));
    assert_eq!(stored.surname.as_deref(), Some("Hollowbrook"));
    assert_eq!(stored.last_synced, Some(100));
    assert_eq!(stored.realm_folder, "93");
}
