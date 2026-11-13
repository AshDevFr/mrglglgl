use std::path::PathBuf;

use mrglglgl_core::db::{
    BackupRetention, CURRENT_VERSION, DATABASE_FILE, Database, DbError, ProviderChoice, Settings,
    WritingSettings,
};
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::prompt::Meddling;
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
        assert_eq!(
            db.settings().unwrap().backup_retention,
            BackupRetention::Days90
        );
        db.save_settings(&Settings {
            wow_root_override: Some(PathBuf::from("/Games/World of Warcraft")),
            backup_retention: BackupRetention::Forever,
            ui_language: Some("fr".into()),
            writing: WritingSettings {
                narrator: "grumpy_dwarf".into(),
                meddling: Meddling::Often,
                provider: ProviderChoice::Endpoint,
                paused: true,
                copy_footer: false,
                ..WritingSettings::default()
            },
            site: Default::default(),
        })
        .unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    let settings = db.settings().unwrap();
    assert_eq!(
        settings.wow_root_override,
        Some(PathBuf::from("/Games/World of Warcraft"))
    );
    assert_eq!(settings.backup_retention, BackupRetention::Forever);
    assert_eq!(settings.ui_language.as_deref(), Some("fr"));
    assert_eq!(settings.writing.narrator, "grumpy_dwarf");
    assert_eq!(settings.writing.meddling, Meddling::Often);
    assert_eq!(settings.writing.provider, ProviderChoice::Endpoint);
    assert!(settings.writing.paused);
    assert!(!settings.writing.copy_footer);

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

/// A database from before backups were kept by age, holding the old copy count.
fn database_with_backup_count(dir: &std::path::Path, count: &str) {
    drop(Database::open(dir).unwrap());
    let conn = rusqlite::Connection::open(dir.join(DATABASE_FILE)).unwrap();
    conn.execute("DELETE FROM settings", []).unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('backup_count', ?1)",
        [count],
    )
    .unwrap();
    // Version 2 is the last one that counted copies; tables and columns added since go too.
    conn.execute_batch(
        "DROP TABLE IF EXISTS continuity; DROP TABLE IF EXISTS stories;
         DROP TABLE IF EXISTS write_queue; DROP TABLE IF EXISTS archive_quarantine;
         DROP TABLE IF EXISTS quest_gists;
         ALTER TABLE archive_days DROP COLUMN format;
         ALTER TABLE archive_journals DROP COLUMN format;
         ALTER TABLE characters DROP COLUMN unused_data;
         ALTER TABLE characters DROP COLUMN writing;
         ALTER TABLE characters DROP COLUMN enabled;",
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 2).unwrap();
}

#[test]
fn writing_defaults_keep_the_game_smooth_and_the_footer_on() {
    let w = WritingSettings::default();
    assert_eq!(w.narrator, "murloc_scribe");
    assert_eq!(w.meddling, Meddling::Sometimes);
    assert_eq!(w.provider, ProviderChoice::Local);
    assert!(!w.write_while_playing, "no writing while the game runs");
    assert!(w.copy_footer);
    assert!(!w.paused);
}

#[test]
fn migrates_the_old_backup_count_to_a_retention_period() {
    for (count, expected) in [
        ("30", BackupRetention::Days90),
        ("10", BackupRetention::Days30),
        ("100", BackupRetention::Year),
    ] {
        let dir = tempfile::tempdir().unwrap();
        database_with_backup_count(dir.path(), count);
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.version().unwrap(), CURRENT_VERSION);
        assert_eq!(
            db.settings().unwrap().backup_retention,
            expected,
            "count {count}"
        );
        let conn = rusqlite::Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
        let left: i64 = conn
            .query_row(
                "SELECT count(*) FROM settings WHERE key = 'backup_count'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(left, 0, "the old setting is gone");
    }
}

#[test]
fn stories_from_before_draft_review_stay_in_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let id = db
        .upsert_character(&source("Wren-Hollowbrook"), None, None)
        .unwrap();
    // A row as an older build wrote it: no status, no note.
    let older = rusqlite::Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
    older
        .execute(
            "INSERT INTO stories (character_id, kind, start_day, end_day, narrator, language,
                 provider, model, prompt_hash, text, created_at)
             VALUES (?1, 'day', '2026-11-05', '2026-11-05', 'murloc_scribe', 'en', 'local',
                 'm', 'h', 'An entry.', 0)",
            [id],
        )
        .unwrap();
    let story = &mrglglgl_core::stories::all(&db, id).unwrap()[0];
    assert_eq!(story.status, mrglglgl_core::stories::StoryStatus::Accepted);
    assert_eq!(story.note, None);
}

#[test]
fn website_settings_default_to_off_with_stand_in_names_and_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let site = db.settings().unwrap().site;
    assert!(!site.enabled);
    assert!(
        site.stand_in_names,
        "other players are hidden unless the player says otherwise"
    );
    assert!(site.hide_realm);
    assert!(site.folder.is_none() && site.characters.is_empty());

    let mut settings = db.settings().unwrap();
    settings.site.enabled = true;
    settings.site.folder = Some(dir.path().join("site"));
    settings.site.title = "Wren's travels".into();
    settings.site.characters = vec![1, 3];
    db.save_settings(&settings).unwrap();
    drop(db);
    let reopened = Database::open(dir.path()).unwrap().settings().unwrap().site;
    assert_eq!(reopened, settings.site);
}

#[test]
fn addon_test_versions_are_off_until_chosen_and_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        assert!(!db.addon_prereleases().unwrap());
        db.set_addon_prereleases(true).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert!(db.addon_prereleases().unwrap());
    db.set_addon_prereleases(false).unwrap();
    assert!(!db.addon_prereleases().unwrap());
}

#[test]
fn a_settings_group_that_cannot_be_read_is_reported_and_kept_aside() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_settings(&Settings::default()).unwrap();
    }
    let raw = rusqlite::Connection::open(dir.path().join("mrglglgl.db")).unwrap();
    raw.execute(
        "UPDATE settings SET value = '{\"paused\": \"not a bool\"' WHERE key = 'writing'",
        [],
    )
    .unwrap();
    raw.execute(
        "UPDATE settings SET value = 'weekly' WHERE key = 'backup_retention'",
        [],
    )
    .unwrap();
    let db = Database::open(dir.path()).unwrap();
    let (settings, unreadable) = db.settings_checked().unwrap();
    assert_eq!(settings, Settings::default());
    assert_eq!(unreadable, vec!["backup_retention", "writing"]);

    // Saving the defaults keeps the text that could not be read, for someone to look at.
    db.save_settings(&settings).unwrap();
    let aside: String = raw
        .query_row(
            "SELECT value FROM settings WHERE key = 'writing.unreadable'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(aside, "{\"paused\": \"not a bool\"");
    assert!(db.settings_checked().unwrap().1.is_empty());
}

#[test]
fn a_character_is_in_the_app_until_the_player_takes_it_out() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source("Wren"), Some(&wren()), None)
        .unwrap();
    assert!(db.characters().unwrap()[0].enabled);
    assert!(!db.is_disabled(&source("Wren")).unwrap());
    // A character the app has never seen is read as usual.
    assert!(!db.is_disabled(&source("Brakka")).unwrap());

    db.set_character_enabled(id, false).unwrap();
    assert!(!db.characters().unwrap()[0].enabled);
    assert!(db.is_disabled(&source("Wren")).unwrap());
    // Reading the character again does not put it back.
    db.upsert_character(&source("Wren"), Some(&wren()), Some(1))
        .unwrap();
    assert!(db.is_disabled(&source("Wren")).unwrap());

    db.set_character_enabled(id, true).unwrap();
    assert!(db.characters().unwrap()[0].enabled);
}

#[test]
fn an_entry_with_no_text_is_dropped_so_its_day_waits_to_be_written() {
    use mrglglgl_core::stories::{self, NewStory, StoryKind, StoryStatus};
    use mrglglgl_core::writer::CheckStatus;
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        let id = db
            .upsert_character(&source("Wren-Hollowbrook"), Some(&wren()), Some(100))
            .unwrap();
        for (day, text) in [("2026-11-04", "Mrgl, a fine day."), ("2026-11-05", "x")] {
            stories::save(
                &db,
                &NewStory {
                    character_id: id,
                    kind: StoryKind::Day,
                    start_day: day.into(),
                    end_day: day.into(),
                    narrator: "murloc_speak".into(),
                    language: "en".into(),
                    provider: "endpoint".into(),
                    model: "a-thinking-model".into(),
                    prompt_hash: "h".into(),
                    text: text.into(),
                    check: CheckStatus::Passed,
                    check_reason: None,
                    status: StoryStatus::Draft,
                    note: None,
                    partial: false,
                },
            )
            .unwrap();
        }
    }
    // As an older app left it: an empty draft, and the schema from before it was cleared.
    let conn = rusqlite::Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
    conn.execute(
        "UPDATE stories SET text = ' ' WHERE start_day = '2026-11-05'",
        [],
    )
    .unwrap();
    conn.execute("DROP TABLE quest_gists", []).unwrap();
    conn.pragma_update(None, "user_version", CURRENT_VERSION - 3)
        .unwrap();
    drop(conn);

    let db = Database::open(dir.path()).unwrap();
    let id = db.characters().unwrap()[0].id;
    assert!(
        stories::versions(&db, id, StoryKind::Day, "2026-11-05")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        stories::versions(&db, id, StoryKind::Day, "2026-11-04")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_set_aside_reason_stored_as_an_english_sentence_keeps_only_what_was_flagged() {
    use mrglglgl_core::stories::{self, NewStory, StoryKind, StoryStatus};
    use mrglglgl_core::writer::CheckStatus;
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        let id = db
            .upsert_character(&source("Wren-Hollowbrook"), Some(&wren()), Some(100))
            .unwrap();
        stories::save(
            &db,
            &NewStory {
                character_id: id,
                kind: StoryKind::Day,
                start_day: "2026-11-07".into(),
                end_day: "2026-11-07".into(),
                narrator: "murloc_speak".into(),
                language: "en".into(),
                provider: "endpoint".into(),
                model: "m".into(),
                prompt_hash: "h".into(),
                text: "Mrgl, I collected ten scorpid tails.".into(),
                check: CheckStatus::Failed,
                check_reason: Some("mentions ten, Booty Bay not in the day's notes".into()),
                status: StoryStatus::Accepted,
                note: None,
                partial: false,
            },
        )
        .unwrap();
    }
    let conn = rusqlite::Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
    conn.execute("DROP TABLE quest_gists", []).unwrap();
    conn.pragma_update(None, "user_version", CURRENT_VERSION - 2)
        .unwrap();
    drop(conn);

    let db = Database::open(dir.path()).unwrap();
    let id = db.characters().unwrap()[0].id;
    let story = &stories::versions(&db, id, StoryKind::Day, "2026-11-07").unwrap()[0];
    assert_eq!(story.check_reason.as_deref(), Some("ten, Booty Bay"));
}

#[test]
fn the_fact_check_asks_again_once_unless_the_player_chose_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.settings().unwrap().writing.check_retries, 1);
    let mut settings = db.settings().unwrap();
    settings.writing.check_retries = 3;
    db.save_settings(&settings).unwrap();
    assert_eq!(db.settings().unwrap().writing.check_retries, 3);
}
