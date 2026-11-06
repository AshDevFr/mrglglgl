use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Local, TimeZone};
use mrglglgl_core::schema::{Day, Event, EventKind, Journal};
use mrglglgl_core::screenshots::{
    folder_for_journal, is_screenshot_file, list, match_to_journal, taken_from_name,
};

/// The file name the game writes for a moment, in this machine's local time.
fn game_name(unix: i64) -> String {
    let local = Local.timestamp_opt(unix, 0).unwrap();
    format!("WoWScrnShot_{}.jpg", local.format("%m%d%y_%H%M%S"))
}

fn local(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> i64 {
    Local
        .with_ymd_and_hms(year, month, day, h, m, s)
        .unwrap()
        .timestamp()
}

fn screenshot_event(time: i64, moment: &str) -> Event {
    Event {
        time,
        zone: Some("Mulgore".into()),
        subzone: None,
        map_id: None,
        level: Some(6),
        kind: EventKind::Screenshot {
            moment: Some(moment.into()),
        },
    }
}

fn journal_with(day: &str, events: Vec<Event>) -> Journal {
    let mut journal = Journal::default();
    journal.days.insert(
        day.into(),
        Day {
            events: Some(events),
            ..Day::default()
        },
    );
    journal
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"\xFF\xD8 not really a jpeg").unwrap();
}

#[test]
fn reads_the_local_time_from_the_game_file_name() {
    let expected = local(2026, 11, 5, 20, 29, 55);
    assert_eq!(
        taken_from_name("WoWScrnShot_110526_202955.jpg"),
        Some(expected)
    );
    assert_eq!(taken_from_name("holiday.jpg"), None);
    assert_eq!(taken_from_name("WoWScrnShot_999999_999999.jpg"), None);
}

#[test]
fn pairs_a_file_with_the_event_a_second_before_it_and_ignores_strays() {
    let dir = tempfile::tempdir().unwrap();
    let event_time = local(2026, 11, 5, 21, 14, 5);
    let near = dir.path().join(game_name(event_time + 1));
    let stray = dir.path().join(game_name(event_time + 11));
    touch(&near);
    touch(&stray);

    let journal = journal_with("2026-11-05", vec![screenshot_event(event_time, "levelUp")]);
    let matched = match_to_journal(&journal, &list(dir.path()));
    let day = &matched["2026-11-05"];
    assert_eq!(day.len(), 1);
    assert_eq!(day[0].path.file_name(), near.file_name());
    assert_eq!(day[0].moment.as_deref(), Some("levelUp"));
}

#[test]
fn uses_each_file_once_and_picks_the_closest() {
    let dir = tempfile::tempdir().unwrap();
    let first = local(2026, 11, 5, 21, 0, 0);
    let second = first + 4;
    touch(&dir.path().join(game_name(first + 1)));
    touch(&dir.path().join(game_name(second + 1)));
    let journal = journal_with(
        "2026-11-05",
        vec![
            screenshot_event(first, "death"),
            screenshot_event(second, "death"),
        ],
    );
    let matched = match_to_journal(&journal, &list(dir.path()));
    let taken: Vec<i64> = matched["2026-11-05"].iter().map(|s| s.taken).collect();
    assert_eq!(taken, [first + 1, second + 1]);
}

#[test]
fn falls_back_to_the_modification_time_for_a_renamed_file() {
    let dir = tempfile::tempdir().unwrap();
    let event_time = local(2026, 11, 5, 21, 14, 5);
    let renamed = dir.path().join("my level up.jpg");
    touch(&renamed);
    File::options()
        .write(true)
        .open(&renamed)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs((event_time + 2) as u64))
        .unwrap();
    let journal = journal_with("2026-11-05", vec![screenshot_event(event_time, "levelUp")]);
    let matched = match_to_journal(&journal, &list(dir.path()));
    assert_eq!(matched["2026-11-05"][0].taken, event_time + 2);
}

#[test]
fn lists_only_images() {
    assert!(is_screenshot_file(Path::new(
        "WoWScrnShot_110526_202955.jpg"
    )));
    assert!(is_screenshot_file(Path::new("shot.TGA")));
    assert!(!is_screenshot_file(Path::new("notes.txt")));
}

#[test]
fn finds_the_flavor_screenshot_folder_from_a_journal_path() {
    let journal = PathBuf::from(
        "/wow/_forever_/WTF/Account/ACCOUNT1/93/Wren-Hollowbrook/SavedVariables/Mrglglgl.lua",
    );
    assert_eq!(
        folder_for_journal(&journal),
        Some(PathBuf::from("/wow/_forever_/Screenshots"))
    );
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

#[test]
fn reading_screenshots_changes_nothing_in_the_game_folder() {
    let wow = tempfile::tempdir().unwrap();
    let shots = wow.path().join("_forever_/Screenshots");
    let event_time = local(2026, 11, 5, 21, 14, 5);
    touch(&shots.join(game_name(event_time + 1)));
    touch(&shots.join("WoWScrnShot_010101_000000.jpg"));
    let before = snapshot(wow.path());
    let journal = journal_with("2026-11-05", vec![screenshot_event(event_time, "bossKill")]);
    let matched = match_to_journal(&journal, &list(&shots));
    assert_eq!(matched["2026-11-05"].len(), 1);
    assert_eq!(snapshot(wow.path()), before);
}
