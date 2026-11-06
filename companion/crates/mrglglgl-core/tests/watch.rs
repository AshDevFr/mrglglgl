use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use mrglglgl_core::watch::{self, is_journal_file, modified_unix};

fn journal_path(root: &Path, character: &str) -> PathBuf {
    root.join("_forever_/WTF/Account/ACCOUNT1/93")
        .join(character)
        .join("SavedVariables/Mrglglgl.lua")
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// macOS reports temp paths under /private; compare by the character folder instead.
fn characters(batch: &BTreeSet<PathBuf>) -> BTreeSet<String> {
    batch
        .iter()
        .map(|p| {
            p.ancestors()
                .nth(2)
                .and_then(|a| a.file_name())
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn reports_a_journal_written_by_the_game_within_five_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let wren = journal_path(dir.path(), "Wren-Hollowbrook");
    write(&wren, "MrglglglCharDB = {}");
    let account = dir.path().join("_forever_/WTF/Account");

    let (sender, receiver) = mpsc::channel();
    let _watcher = watch::watch(
        &[account],
        Duration::from_millis(300),
        is_journal_file,
        move |batch| sender.send(batch).unwrap(),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(300));

    let started = Instant::now();
    // The game writes the file in several steps; they arrive as one change.
    for step in 0..5 {
        write(
            &wren,
            &format!("MrglglglCharDB = {{ [\"step\"] = {step} }}"),
        );
        std::thread::sleep(Duration::from_millis(30));
    }
    write(
        &wren.with_file_name("Questie.lua"),
        "-- another addon's file is not a journal",
    );

    let batch = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a change within five seconds");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(
        characters(&batch),
        BTreeSet::from(["Wren-Hollowbrook".to_string()])
    );
    assert!(batch.iter().all(|p| is_journal_file(p)));
}

#[test]
fn reports_each_character_that_changed() {
    let dir = tempfile::tempdir().unwrap();
    let account = dir.path().join("_forever_/WTF/Account");
    // Character folders exist before the app starts watching, as in a real install: the game
    // creates them with the character. (Linux registers new subfolders asynchronously, so a file
    // written into a folder created after watching began can be missed there.)
    for character in ["Wren-Hollowbrook", "Tamsin-Reedwater"] {
        fs::create_dir_all(journal_path(dir.path(), character).parent().unwrap()).unwrap();
    }
    let (sender, receiver) = mpsc::channel();
    let _watcher = watch::watch(
        &[account],
        Duration::from_millis(300),
        is_journal_file,
        move |batch| sender.send(batch).unwrap(),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(300));

    write(&journal_path(dir.path(), "Wren-Hollowbrook"), "A = {}");
    write(&journal_path(dir.path(), "Tamsin-Reedwater"), "B = {}");

    let mut seen = BTreeSet::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while seen.len() < 2 && Instant::now() < deadline {
        if let Ok(batch) = receiver.recv_timeout(Duration::from_millis(500)) {
            seen.extend(characters(&batch));
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from([
            "Tamsin-Reedwater".to_string(),
            "Wren-Hollowbrook".to_string()
        ])
    );
}

#[test]
fn last_synced_is_the_file_modification_time() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Mrglglgl.lua");
    write(&path, "A = {}");
    let synced = modified_unix(&path).unwrap();
    assert!((watch::now_unix() - synced).abs() < 60);
    assert_eq!(modified_unix(&dir.path().join("missing.lua")), None);
}

#[test]
fn refuses_a_folder_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let result = watch::watch(
        &[dir.path().join("missing")],
        Duration::from_millis(100),
        is_journal_file,
        |_| {},
    );
    assert!(result.is_err());
}
