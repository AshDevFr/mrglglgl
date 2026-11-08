use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Days, Local, NaiveDate, TimeZone};
use mrglglgl_core::archive::{
    self, ImportOutcome, backup_folder, backup_journal, backups, backups_to_remove, merge_snapshot,
};
use mrglglgl_core::db::{BackupRetention, Database};
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
            // Files only: a folder's own timestamp is updated lazily on some file systems (NTFS),
            // so it can move after the test's setup writes without the code under test writing.
            if meta.is_dir() {
                stack.push(entry.path());
                out.push((entry.path(), 0, None));
            } else {
                out.push((entry.path(), meta.len(), meta.modified().ok()));
            }
        }
    }
    out.sort();
    out
}

fn synthetic() -> Journal {
    schema::journal_from_text(&shared("synthetic-character.lua")).unwrap()
}

/// Noon on 2026-11-06, local time: the fake "now" for retention tests.
fn now() -> i64 {
    Local
        .with_ymd_and_hms(2026, 11, 6, 12, 0, 0)
        .unwrap()
        .timestamp()
}

/// A local time `days` days before 2026-11-06, at `hour`:00.
fn days_ago(days: u64, hour: u32) -> i64 {
    let date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap() - Days::new(days);
    Local
        .from_local_datetime(&date.and_hms_opt(hour, 0, 0).unwrap())
        .earliest()
        .unwrap()
        .timestamp()
}

/// The game's journal history: three saves a day over the last week, and two saves a day on
/// every tenth day going back more than a year.
fn history() -> Vec<i64> {
    let mut times = Vec::new();
    let older: Vec<u64> = (7..=397).step_by(10).collect();
    for &days in older.iter().rev() {
        times.push(days_ago(days, 9));
        times.push(days_ago(days, 21));
    }
    for days in (0..7).rev() {
        for hour in [9, 11, 13] {
            if days == 0 && hour > 12 {
                continue; // not yet "now"
            }
            times.push(days_ago(days, hour));
        }
    }
    times
}

fn names(times: &[i64]) -> Vec<PathBuf> {
    times
        .iter()
        .map(|t| PathBuf::from(format!("{t:012}.lua")))
        .collect()
}

/// What should survive: every copy from the last week, the last copy of each older day, and
/// nothing older than the period.
fn expected_kept(period_days: Option<u64>) -> Vec<i64> {
    let mut kept: Vec<i64> = (7..=397)
        .step_by(10)
        .filter(|days| period_days.is_none_or(|p| *days <= p))
        .map(|days| days_ago(days, 21))
        .collect();
    for days in 0..7 {
        for hour in [9, 11, 13] {
            if days == 0 && hour > 12 {
                continue;
            }
            kept.push(days_ago(days, hour));
        }
    }
    kept.sort();
    kept
}

fn stamp(path: &Path) -> i64 {
    path.file_stem().unwrap().to_str().unwrap().parse().unwrap()
}

fn kept_after_thinning(retention: BackupRetention) -> Vec<i64> {
    let all = names(&history());
    let removed = backups_to_remove(&all, now(), retention);
    let mut kept: Vec<i64> = all
        .iter()
        .filter(|p| !removed.contains(p))
        .map(|p| stamp(p))
        .collect();
    kept.sort();
    kept
}

#[test]
fn keeps_thirty_days_of_backups() {
    assert_eq!(
        kept_after_thinning(BackupRetention::Days30),
        expected_kept(Some(30))
    );
}

#[test]
fn keeps_ninety_days_of_backups_by_default() {
    assert_eq!(BackupRetention::default(), BackupRetention::Days90);
    assert_eq!(
        kept_after_thinning(BackupRetention::Days90),
        expected_kept(Some(90))
    );
}

#[test]
fn keeps_a_year_of_backups() {
    assert_eq!(
        kept_after_thinning(BackupRetention::Year),
        expected_kept(Some(365))
    );
}

#[test]
fn keeps_backups_forever_but_still_thins_older_days() {
    let kept = kept_after_thinning(BackupRetention::Forever);
    assert_eq!(kept, expected_kept(None));
    // One copy per older day: the 09:00 save of each of those days is gone.
    assert!(!kept.contains(&days_ago(397, 9)));
    assert!(kept.contains(&days_ago(397, 21)));
}

#[test]
fn never_removes_the_newest_copy_however_old() {
    let all = names(&[days_ago(800, 9), days_ago(700, 9)]);
    let removed = backups_to_remove(&all, now(), BackupRetention::Days30);
    assert_eq!(removed, names(&[days_ago(800, 9)]));
}

#[test]
fn thins_real_backups_and_never_touches_the_game_folder() {
    let wow = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let wren = source(wow.path(), "Wren-Hollowbrook");
    let mut game_state = Vec::new();
    for (i, time) in history().into_iter().enumerate() {
        game_writes(
            &wren.journal_path,
            &format!("MrglglglCharDB = {{ [\"schema\"] = 1, [\"change\"] = {i} }}"),
            time as u64,
        );
        game_state = snapshot(wow.path());
        // Each save is backed up at the moment the game writes it.
        backup_journal(data.path(), &wren, BackupRetention::Days90, time).unwrap();
        assert_eq!(snapshot(wow.path()), game_state, "the game folder changed");
    }
    // Opening the app at "now" thins what aged since.
    backup_journal(data.path(), &wren, BackupRetention::Days90, now()).unwrap();

    let kept: Vec<i64> = backups(&backup_folder(data.path(), &wren))
        .iter()
        .map(|p| stamp(p))
        .collect();
    assert_eq!(kept, expected_kept(Some(90)));
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
    let first = backup_journal(data.path(), &wren, BackupRetention::Days90, now()).unwrap();
    game_writes(
        &wren.journal_path,
        &shared("real-session.lua"),
        1_794_000_500,
    );
    let second = backup_journal(data.path(), &wren, BackupRetention::Days90, now()).unwrap();
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
        newer.days["2026-11-08"].digest.as_ref().unwrap().version,
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
fn quest_text_the_game_has_pruned_stays_in_the_archive() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    let earlier = synthetic();
    let mut pruned = synthetic();
    let quest = pruned.quests.get_mut(&749).unwrap();
    quest.text = None;
    quest.last = Some("2026-11-20".into());

    merge_snapshot(&db, id, &earlier, 100).unwrap();
    merge_snapshot(&db, id, &pruned, 200).unwrap();

    let archived = &archive::load(&db, id).unwrap().unwrap().quests[&749];
    assert_eq!(archived.text, earlier.quests[&749].text);
    assert_eq!(archived.last.as_deref(), Some("2026-11-20"));
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

    let outcome = archive::import(&db, data.path(), &wren, BackupRetention::Days90).unwrap();
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

    let outcome = archive::import(&db, data.path(), &wren, BackupRetention::Days90).unwrap();
    assert!(matches!(
        outcome,
        ImportOutcome::Unreadable {
            error: JournalError::NewerSchema { found: 99, .. },
            ..
        }
    ));
    assert!(matches!(
        archive::import(&db, data.path(), &tamsin, BackupRetention::Days90).unwrap(),
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
        archive::import(&db, data.path(), &wren, BackupRetention::Days90).unwrap()
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
        archive::import(&db, data.path(), &wren, BackupRetention::Days90).unwrap()
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

#[test]
fn days_archived_before_professions_were_recorded_still_load_and_merge() {
    // An older build stored days and digests without the profession fields. Reading them must
    // not fail, or the whole journal stops loading and no newer snapshot can be merged in.
    let data = tempfile::tempdir().unwrap();
    let db = Database::open(data.path()).unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    merge_snapshot(&db, id, &synthetic(), 100).unwrap();
    let date = synthetic().days.keys().next().unwrap().clone();
    let legacy = r#"{"events":[],"kills":{},"anonKills":0,"digest":{"version":1,
        "counts":{"sessions":1,"levels":0,"accepted":0,"turnedIn":0,"deaths":0,"fish":0,
        "junk":0,"boots":0,"loot":0,"discoveries":0,"bosses":0,"group":0,"screenshots":0,
        "kills":0,"murlocs":0,"anonKills":0},"levelFrom":null,"levelTo":null,"zones":[],
        "subzones":[],"discovered":[],"accepted":[],"turnedIn":[],"deaths":[],"kills":[],
        "notableKills":[],"fish":[],"loot":[],"companions":[],"bosses":[],"onlyFished":false,
        "nothingHappened":true}}"#;
    let older_build = rusqlite::Connection::open(data.path().join("mrglglgl.db")).unwrap();
    older_build
        .execute(
            "UPDATE archive_days SET data_json = ?1 WHERE character_id = ?2 AND day = ?3",
            rusqlite::params![legacy, id, date],
        )
        .unwrap();

    let loaded = archive::load(&db, id).unwrap().unwrap();
    assert!(loaded.days[&date].skills.is_empty());
    merge_snapshot(&db, id, &synthetic(), 200).unwrap();
}

/// A journal with one day, 2026-11-08 from the shared fixture, and that day's stats.
fn one_day() -> (Database, i64, Journal) {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    let mut j = synthetic();
    j.days.retain(|d, _| d == "2026-11-08");
    (db, id, j)
}

#[test]
fn a_compacted_snapshot_never_drops_professions_the_archive_holds() {
    let (db, id, raw) = one_day();
    let day = &raw.days["2026-11-08"];
    assert!(!day.skills.is_empty() && !day.crafted.is_empty() && !day.gathered.is_empty());
    let mut compacted = raw.clone();
    let d = compacted.days.get_mut("2026-11-08").unwrap();
    d.events = None;
    d.kills.clear();
    d.anon_kills = 0;
    d.skills.clear();
    d.crafted.clear();
    d.gathered.clear();
    d.digest = Some(schema::Digest::default());

    merge_snapshot(&db, id, &raw, 100).unwrap();
    merge_snapshot(&db, id, &compacted, 200).unwrap();
    let kept = &archive::load(&db, id).unwrap().unwrap().days["2026-11-08"];
    assert_eq!(kept.skills, day.skills);
    assert_eq!(kept.crafted, day.crafted);
    assert_eq!(kept.gathered, day.gathered);
    assert_eq!(kept.events, day.events);
}

#[test]
fn money_forgotten_in_the_game_stays_in_the_archive_and_new_money_adds() {
    let (db, id, raw) = one_day();
    let day = &raw.days["2026-11-08"];
    assert!(!day.money.is_empty());
    let mut later = raw.clone();
    let d = later.days.get_mut("2026-11-08").unwrap();
    // Forget cleared the day's money, then the player sold something else.
    d.money = schema::Money::default();
    d.money.inc.insert("vendor".into(), 500);

    merge_snapshot(&db, id, &raw, 100).unwrap();
    merge_snapshot(&db, id, &later, 200).unwrap();
    let kept = &archive::load(&db, id).unwrap().unwrap().days["2026-11-08"];
    assert_eq!(kept.money.inc["auction"], 1840);
    assert_eq!(kept.money.inc["vendor"], 500);
    assert_eq!(kept.money.out, day.money.out);
}

#[test]
fn chat_the_game_dropped_stays_in_the_archive_and_new_lines_join_it() {
    let (db, id, raw) = one_day();
    let day = &raw.days["2026-11-08"];
    assert_eq!(day.chat.len(), 3);
    let mut later = raw.clone();
    let d = later.days.get_mut("2026-11-08").unwrap();
    // The game dropped the first lines (Forget), and one more was said.
    let mut extra = d.chat[0].clone();
    extra.time += 3600;
    extra.text = "Back to Bloodhoof, then.".into();
    d.chat = vec![d.chat[2].clone(), extra.clone()];

    merge_snapshot(&db, id, &raw, 100).unwrap();
    merge_snapshot(&db, id, &later, 200).unwrap();
    merge_snapshot(&db, id, &later, 300).unwrap();
    let kept = &archive::load(&db, id).unwrap().unwrap().days["2026-11-08"];
    assert_eq!(kept.chat.len(), 4, "each line once");
    assert_eq!(kept.chat[..3], day.chat[..]);
    assert_eq!(kept.chat[3], extra);
}

#[test]
fn a_thinned_digest_never_replaces_a_fuller_one() {
    let (db, id, mut j) = one_day();
    let d = j.days.get_mut("2026-11-08").unwrap();
    d.events = None;
    let full = schema::Digest {
        zones: vec!["Mulgore".into(), "The Barrens".into(), "Durotar".into()],
        counts: schema::DigestCounts {
            kills: 40,
            ..Default::default()
        },
        ..Default::default()
    };
    d.digest = Some(full.clone());
    let mut thin = j.clone();
    let t = thin.days.get_mut("2026-11-08").unwrap();
    let mut thinned = full.clone();
    thinned.zones.truncate(1);
    thinned.thin = true;
    t.digest = Some(thinned);

    merge_snapshot(&db, id, &j, 100).unwrap();
    merge_snapshot(&db, id, &thin, 200).unwrap();
    let kept = &archive::load(&db, id).unwrap().unwrap().days["2026-11-08"];
    assert_eq!(kept.digest.as_ref().unwrap(), &full);
}

#[test]
fn an_old_file_with_a_new_date_never_erases_newer_data() {
    // A restored backup or a synced folder gives old content a fresh modification time.
    let (db, id, newer) = one_day();
    let mut older = newer.clone();
    let d = older.days.get_mut("2026-11-08").unwrap();
    d.events.as_mut().unwrap().truncate(3);
    d.kills.clear();
    older.stats.boots = 0;
    older.stats.murlocs_slain = 0;
    for c in older.companions.values_mut() {
        c.days = 0;
    }

    merge_snapshot(&db, id, &newer, 100).unwrap();
    merge_snapshot(&db, id, &older, 300).unwrap();
    let kept = archive::load(&db, id).unwrap().unwrap();
    assert_eq!(
        kept.days["2026-11-08"].events,
        newer.days["2026-11-08"].events
    );
    assert_eq!(
        kept.days["2026-11-08"].kills,
        newer.days["2026-11-08"].kills
    );
    assert_eq!(kept.stats, newer.stats, "stats never go back");
    assert_eq!(kept.companions, newer.companions);
}

#[test]
fn forgetting_in_the_game_leaves_the_archive_whole() {
    // The addon's Forget shrinks the game file only; the app keeps its own copy.
    let (db, id, before) = one_day();
    let mut forgot = before.clone();
    let d = forgot.days.get_mut("2026-11-08").unwrap();
    d.kills.clear();
    d.anon_kills = 0;
    d.events
        .as_mut()
        .unwrap()
        .retain(|e| !matches!(e.kind, schema::EventKind::Death { .. }));

    merge_snapshot(&db, id, &before, 100).unwrap();
    merge_snapshot(&db, id, &forgot, 200).unwrap();
    let kept = &archive::load(&db, id).unwrap().unwrap().days["2026-11-08"];
    assert_eq!(kept.kills, before.days["2026-11-08"].kills);
    assert_eq!(kept.events, before.days["2026-11-08"].events);
}

fn archived(dir: &Path) -> (Database, i64, String) {
    let db = Database::open(dir).unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    merge_snapshot(&db, id, &synthetic(), 100).unwrap();
    (db, id, "2026-11-08".to_string())
}

fn raw(dir: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(dir.join("mrglglgl.db")).unwrap()
}

#[test]
fn one_unreadable_day_never_hides_the_rest_of_a_character() {
    let dir = tempfile::tempdir().unwrap();
    let (db, id, day) = archived(dir.path());
    raw(dir.path())
        .execute(
            "UPDATE archive_days SET data_json = '{not json' WHERE day = ?1",
            [&day],
        )
        .unwrap();
    let loaded = archive::load_checked(&db, id).unwrap().unwrap();
    assert_eq!(loaded.unreadable, vec![day.clone()]);
    assert!(!loaded.journal.days.contains_key(&day));
    assert_eq!(loaded.journal.days.len(), synthetic().days.len() - 1);
    assert!(
        archive::load(&db, id).unwrap().is_some(),
        "the plain load still works"
    );
}

#[test]
fn a_day_saved_by_a_newer_app_is_skipped_not_misread() {
    let dir = tempfile::tempdir().unwrap();
    let (db, id, day) = archived(dir.path());
    raw(dir.path())
        .execute(
            "UPDATE archive_days SET format = ?1 WHERE day = ?2",
            rusqlite::params![archive::ARCHIVE_FORMAT + 1, day],
        )
        .unwrap();
    let loaded = archive::load_checked(&db, id).unwrap().unwrap();
    assert_eq!(loaded.unreadable, vec![day]);
}

#[test]
fn an_unreadable_day_is_set_aside_before_a_snapshot_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let (db, id, day) = archived(dir.path());
    raw(dir.path())
        .execute(
            "UPDATE archive_days SET data_json = '{not json' WHERE day = ?1",
            [&day],
        )
        .unwrap();
    merge_snapshot(&db, id, &synthetic(), 200).unwrap();
    let kept: String = raw(dir.path())
        .query_row(
            "SELECT data_json FROM archive_quarantine WHERE day = ?1",
            [&day],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(kept, "{not json");
    let loaded = archive::load_checked(&db, id).unwrap().unwrap();
    assert!(loaded.unreadable.is_empty());
    assert!(loaded.journal.days.contains_key(&day));
}

#[test]
fn an_archived_day_in_format_one_still_reads_every_field() {
    // Written by the app in archive format 1. A change to the archived types that would lose or
    // misread any of it fails here, before it reaches anyone's archive.
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/archive-day-format-1.json");
    if std::env::var_os("MRGLGLGL_UPDATE_SNAPSHOTS").is_some() && !path.exists() {
        let day = &synthetic().days["2026-11-08"];
        fs::write(&path, serde_json::to_string_pretty(day).unwrap()).unwrap();
    }
    let text = fs::read_to_string(&path).unwrap();
    let original: serde_json::Value = serde_json::from_str(&text).unwrap();
    let day: schema::Day = serde_json::from_str(&text).unwrap();
    // Every value the old row held reads back the same; fields added since may appear beside them.
    assert_kept(&original, &serde_json::to_value(&day).unwrap(), "day");
}

fn assert_kept(old: &serde_json::Value, new: &serde_json::Value, at: &str) {
    match (old, new) {
        (serde_json::Value::Object(o), serde_json::Value::Object(n)) => {
            for (key, value) in o {
                let path = format!("{at}.{key}");
                let read = n
                    .get(key)
                    .unwrap_or_else(|| panic!("{path} was not read back"));
                assert_kept(value, read, &path);
            }
        }
        (serde_json::Value::Array(o), serde_json::Value::Array(n)) => {
            assert_eq!(o.len(), n.len(), "{at} changed length");
            for (i, (a, b)) in o.iter().zip(n).enumerate() {
                assert_kept(a, b, &format!("{at}[{i}]"));
            }
        }
        _ => assert_eq!(old, new, "{at} reads back differently"),
    }
}

#[test]
fn the_database_is_snapshotted_weekly_and_four_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _, _) = archived(dir.path());
    let start = 1_794_000_000; // 2026-11-07
    let first = db
        .snapshot_if_due(dir.path(), start)
        .unwrap()
        .expect("a first snapshot");
    assert!(first.is_file());
    assert!(
        db.snapshot_if_due(dir.path(), start + 3 * 86_400)
            .unwrap()
            .is_none()
    );
    for week in 1..=5 {
        assert!(
            db.snapshot_if_due(dir.path(), start + week * 7 * 86_400)
                .unwrap()
                .is_some()
        );
    }
    let kept = fs::read_dir(dir.path().join("snapshots")).unwrap().count();
    assert_eq!(kept, 4);
    // A snapshot is a whole database, archive included.
    let newest = fs::read_dir(dir.path().join("snapshots"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .max()
        .unwrap();
    let days: i64 = rusqlite::Connection::open(newest)
        .unwrap()
        .query_row("SELECT count(*) FROM archive_days", [], |row| row.get(0))
        .unwrap();
    assert_eq!(days as usize, synthetic().days.len());
}

#[test]
fn the_addons_notes_are_read_without_the_days() {
    let dir = tempfile::tempdir().unwrap();
    let (db, id, _) = archived(dir.path());
    let journal = synthetic();
    let (heartbeat, errors) = archive::addon_notes(&db, id).unwrap();
    assert_eq!(heartbeat, journal.addon);
    assert_eq!(errors, journal.errors);
    assert!(heartbeat.is_some(), "the sample journal notes its addon");
    assert_eq!(archive::addon_notes(&db, id + 1).unwrap(), (None, None));
}

#[test]
fn the_current_level_is_the_newest_days_highest() {
    use mrglglgl_core::schema::{Event, EventKind};
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    assert_eq!(archive::current_level(&db, id).unwrap(), None);

    let mut journal = synthetic();
    let newest = journal.days.keys().last().unwrap().clone();
    let event = |time, level| Event {
        time,
        zone: None,
        subzone: None,
        map_id: None,
        level: Some(level),
        balance: None,
        kind: EventKind::Level,
    };
    // The logout can carry the level from before the session's level-ups.
    journal.days.get_mut(&newest).unwrap().events =
        Some(vec![event(1, 5), event(2, 7), event(3, 6)]);
    merge_snapshot(&db, id, &journal, 100).unwrap();
    assert_eq!(archive::current_level(&db, id).unwrap(), Some(7));

    // A day that only ever arrived compacted keeps its level in the digest.
    let penny = db
        .upsert_character(&source(Path::new("/wow"), "Penny"), None, None)
        .unwrap();
    let day = journal.days.get_mut(&newest).unwrap();
    day.events = None;
    day.digest.get_or_insert_with(Default::default).level_to = Some(8);
    merge_snapshot(&db, penny, &journal, 100).unwrap();
    assert_eq!(archive::current_level(&db, penny).unwrap(), Some(8));
}

#[test]
fn time_grouped_keeps_the_longest_count_of_each_companion() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(&source(Path::new("/wow"), "Wren-Hollowbrook"), None, None)
        .unwrap();
    let mut early = synthetic();
    let day = early.days.keys().last().unwrap().clone();
    early.days.get_mut(&day).unwrap().grouped = [("Tamsin-Reedwater".to_string(), 600)]
        .into_iter()
        .collect();
    let mut later = early.clone();
    later.days.get_mut(&day).unwrap().grouped = [
        ("Tamsin-Reedwater".to_string(), 1800),
        ("Odo".to_string(), 60),
    ]
    .into_iter()
    .collect();
    merge_snapshot(&db, id, &early, 100).unwrap();
    merge_snapshot(&db, id, &later, 200).unwrap();
    // An older snapshot read again changes nothing.
    merge_snapshot(&db, id, &early, 300).unwrap();
    let grouped = archive::load(&db, id).unwrap().unwrap().days[&day]
        .grouped
        .clone();
    assert_eq!(grouped.get("Tamsin-Reedwater"), Some(&1800));
    assert_eq!(grouped.get("Odo"), Some(&60));
}
