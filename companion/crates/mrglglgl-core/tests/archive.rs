use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mrglglgl_core::archive::{
    self, ImportOutcome, backup_folder, backup_journal, backups, merge_snapshot,
};
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::schema::{self, Journal, JournalError};

fn shared(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn source(root: &Path, character: &str) -> CharacterSource {
    CharacterSource {
        flavor: "_forever_".into(),
        account: "ACCOUNT1".into(),
        realm_folder: "93".into(),
        character_folder: character.into(),
        journal_path: root
            .join("_forever_/WTF/Account/ACCOUNT1/93")
            .join(character)
            .join("SavedVariables/Mrglglgl.lua"),
    }
}

/// Writes a journal as the game would, with a chosen modification time.
fn game_writes(path: &Path, text: &str, mtime: u64) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(mtime))
        .unwrap();
}

fn snapshot(path: &Path) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    let mut out = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                stack.push(entry.path());
            }
            out.push((entry.path(), meta.len(), meta.modified().ok()));
        }
    }
    out.sort();
    out
}

fn synthetic() -> Journal {
    schema::journal_from_text(&shared("synthetic-character.lua")).unwrap()
}

#[test]
fn keeps_the_newest_backups_and_never_touches_the_game_folder() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    let base = 1_794_000_000;
    let mut game_state = Vec::new();
    for i in 0..35 {
        game_writes(
            &wren.journal_path,
            &format!("MrglglglCharDB = {{ [\"schema\"] = 1, [\"change\"] = {i} }}"),
            base + i,
        );
        game_state = snapshot(wow.path());
        backup_journal(data.path(), &wren, 30).unwrap();
        assert_eq!(snapshot(wow.path()), game_state, "the game folder changed");
    }
    let kept = backups(&backup_folder(data.path(), &wren));
    assert_eq!(kept.len(), 30);
    assert!(
        kept.first()
            .unwrap()
            .ends_with(format!("{:012}.lua", base + 5))
    );
    assert!(
        kept.last()
            .unwrap()
            .ends_with(format!("{:012}.lua", base + 34))
    );
    assert_eq!(
        fs::read_to_string(kept.last().unwrap()).unwrap(),
        "MrglglglCharDB = { [\"schema\"] = 1, [\"change\"] = 34 }"
    );
    assert_eq!(snapshot(wow.path()), game_state);
}

#[test]
fn skips_a_backup_identical_to_the_newest() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    game_writes(
        &wren.journal_path,
        &shared("real-session.lua"),
        1_794_000_000,
    );
    let first = backup_journal(data.path(), &wren, 30).unwrap();
    game_writes(
        &wren.journal_path,
        &shared("real-session.lua"),
        1_794_000_500,
    );
    let second = backup_journal(data.path(), &wren, 30).unwrap();
    assert_eq!(first, second);
    assert_eq!(backups(&backup_folder(data.path(), &wren)).len(), 1);
}

#[test]
fn keeps_raw_events_a_later_snapshot_compacted_away() {
    let db = Database::open_in_memory().unwrap();
    let wren = source(Path::new("/wow"), "Wren-Hollowbrook");
    let id = db.upsert_character(&wren, None, None).unwrap();

    // Snapshot A: the 2026-11-08 day still has its raw events.
    let a = synthetic();
    let raw_day = a.days["2026-11-08"].clone();
    assert!(raw_day.events.is_some());

    // Snapshot B, written later: the addon has compacted that day to its digest.
    let mut b = a.clone();
    let compacted = b.days.get_mut("2026-11-08").unwrap();
    compacted.events = None;
    compacted.kills.clear();
    compacted.anon_kills = 0;
    let mut newer_digest = compacted.digest.clone().unwrap();
    newer_digest.version = 2;
    compacted.digest = Some(newer_digest);

    merge_snapshot(&db, id, &a, 100).unwrap();
    merge_snapshot(&db, id, &b, 200).unwrap();

    let day = &archive::load(&db, id).unwrap().unwrap().days["2026-11-08"];
    assert_eq!(day.digest.as_ref().unwrap().version, 2, "the digest from B");
    assert_eq!(day.events, raw_day.events, "the raw events from A");
    assert_eq!(day.kills, raw_day.kills);
    assert_eq!(day.anon_kills, raw_day.anon_kills);
}

#[test]
fn an_older_snapshot_read_late_only_fills_gaps() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    let newer = synthetic();
    let mut older = synthetic();
    older.stats.boots = 0;
    let older_day = older.days.get_mut("2026-11-08").unwrap();
    older_day.digest.as_mut().unwrap().version = 0;

    // The compacted 2026-11-01 day: the older snapshot still had raw events for it.
    let mut older_raw = older.days["2026-11-08"].clone();
    older_raw.digest = None;
    older.days.insert("2026-11-01".into(), older_raw.clone());

    merge_snapshot(&db, id, &newer, 200).unwrap();
    merge_snapshot(&db, id, &older, 100).unwrap();

    let journal = archive::load(&db, id).unwrap().unwrap();
    assert_eq!(journal.stats.boots, 1, "stats from the newer snapshot");
    assert_eq!(
        journal.days["2026-11-08"].digest.as_ref().unwrap().version,
        1,
        "the newer digest is not replaced by an older one"
    );
    let filled = &journal.days["2026-11-01"];
    assert_eq!(
        filled.events, older_raw.events,
        "missing raw events filled in"
    );
    assert_eq!(
        filled.digest, newer.days["2026-11-01"].digest,
        "the newer digest kept"
    );
}

#[test]
fn re_importing_the_same_snapshot_changes_nothing() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    merge_snapshot(&db, id, &synthetic(), 100).unwrap();
    let once = archive::load(&db, id).unwrap();
    merge_snapshot(&db, id, &synthetic(), 100).unwrap();
    assert_eq!(archive::load(&db, id).unwrap(), once);
    assert_eq!(once.unwrap().days.len(), synthetic().days.len());
}

#[test]
fn imports_from_the_backup_and_records_the_character() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let db = Database::open(data.path()).unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    game_writes(
        &wren.journal_path,
        &shared("synthetic-character.lua"),
        1_794_400_000,
    );
    let before = snapshot(wow.path());

    let outcome = archive::import(&db, data.path(), &wren, 30).unwrap();
    let ImportOutcome::Imported { character_id } = outcome else {
        panic!("not imported: {outcome:?}");
    };
    assert_eq!(snapshot(wow.path()), before, "the game folder changed");

    let stored = db.character(character_id).unwrap().unwrap();
    assert_eq!(stored.name.as_deref(), Some("Wren"));
    assert_eq!(stored.surname.as_deref(), Some("Hollowbrook"));
    assert_eq!(stored.last_synced, Some(1_794_400_000));
    assert_eq!(stored.status, None);
    assert_eq!(
        archive::load(&db, character_id).unwrap().unwrap(),
        synthetic()
    );
}

#[test]
fn keeps_a_character_whose_journal_is_too_new_and_still_backs_it_up() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let db = Database::open(data.path()).unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    let tamsin = source(wow.path(), "Tamsin-Reedwater");
    game_writes(
        &wren.journal_path,
        "\nMrglglglCharDB = {\n[\"schema\"] = 99,\n}\n",
        1,
    );
    game_writes(&tamsin.journal_path, &shared("real-session.lua"), 2);

    let outcome = archive::import(&db, data.path(), &wren, 30).unwrap();
    assert!(matches!(
        outcome,
        ImportOutcome::Unreadable {
            error: JournalError::NewerSchema { found: 99, .. },
            ..
        }
    ));
    assert!(matches!(
        archive::import(&db, data.path(), &tamsin, 30).unwrap(),
        ImportOutcome::Imported { .. }
    ));
    let characters = db.characters().unwrap();
    let statuses: Vec<_> = characters.iter().map(|c| c.status.as_deref()).collect();
    assert!(statuses.contains(&Some("newer")));
    assert!(statuses.contains(&None));
    assert_eq!(backups(&backup_folder(data.path(), &wren)).len(), 1);
}

#[test]
fn deleting_a_character_removes_its_archive_and_backups_but_not_the_game_file() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let db = Database::open(data.path()).unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    game_writes(&wren.journal_path, &shared("real-session.lua"), 1);
    let ImportOutcome::Imported { character_id } =
        archive::import(&db, data.path(), &wren, 30).unwrap()
    else {
        panic!("not imported");
    };

    archive::delete_character(&db, data.path(), character_id).unwrap();
    assert!(db.characters().unwrap().is_empty());
    assert_eq!(archive::load(&db, character_id).unwrap(), None);
    assert!(!backup_folder(data.path(), &wren).exists());
    assert!(wren.journal_path.is_file(), "the game's file stays");
}

#[test]
fn a_character_whose_game_files_are_gone_stays_readable() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let db = Database::open(data.path()).unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    game_writes(&wren.journal_path, &shared("synthetic-character.lua"), 1);
    let ImportOutcome::Imported { character_id } =
        archive::import(&db, data.path(), &wren, 30).unwrap()
    else {
        panic!("not imported");
    };
    drop(wow);
    assert!(!wren.journal_path.exists());
    assert_eq!(
        archive::load(&db, character_id).unwrap().unwrap(),
        synthetic()
    );
}
