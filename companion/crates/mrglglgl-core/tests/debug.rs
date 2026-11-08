use std::path::PathBuf;

use mrglglgl_core::archive;
use mrglglgl_core::db::Database;
use mrglglgl_core::debug;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::narrators::Library;
use mrglglgl_core::schema::{Journal, journal_from_text};
use mrglglgl_core::stories::{self, NewStory, StoryKind, StoryStatus};
use mrglglgl_core::writer::CheckStatus;

fn journal() -> Journal {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap()
}

const DAY: &str = "2026-11-08";

fn setup() -> (Database, i64) {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(
            &CharacterSource {
                flavor: "_forever_".into(),
                account: "ACCOUNT1".into(),
                realm_folder: "93".into(),
                character_folder: "Wren".into(),
                journal_path: PathBuf::from("/wow/Mrglglgl.lua"),
            },
            None,
            None,
        )
        .unwrap();
    archive::merge_snapshot(&db, id, &journal(), 1).unwrap();
    (db, id)
}

#[test]
fn a_days_debug_file_holds_what_was_recorded_told_and_written() {
    let (db, id) = setup();
    stories::save(
        &db,
        &NewStory {
            character_id: id,
            kind: StoryKind::Day,
            start_day: DAY.into(),
            end_day: DAY.into(),
            narrator: "grumpy_dwarf".into(),
            language: "en".into(),
            provider: "local".into(),
            model: "qwen3.5-4b".into(),
            prompt_hash: "h".into(),
            text: "Mine work.".into(),
            check: CheckStatus::Rewritten,
            check_reason: None,
            status: StoryStatus::Accepted,
            note: None,
            partial: false,
        },
    )
    .unwrap();

    let file = debug::day(&db, &Library::default(), id, StoryKind::Day, DAY, "en").unwrap();

    assert_eq!(file.start_day, DAY);
    assert!(file.recorded.is_some(), "the day as the addon recorded it");
    assert!(!file.facts.is_empty(), "the facts the narrator is told");
    let prompt = file
        .prompt
        .as_ref()
        .expect("the prompt the next write gets");
    for fact in &file.facts {
        assert!(prompt.user.contains(fact.as_str()), "{fact}");
    }
    assert!(prompt.words > 0);
    assert_eq!(file.versions.len(), 1);
    assert_eq!(file.versions[0].model, "qwen3.5-4b");

    // It reads back as JSON a person or a tool can open.
    let json = serde_json::to_value(&file).unwrap();
    assert_eq!(json["versions"][0]["text"], "Mine work.");
}

#[test]
fn a_character_without_a_journal_has_no_debug_file() {
    let db = Database::open_in_memory().unwrap();
    assert!(debug::day(&db, &Library::default(), 99, StoryKind::Day, DAY, "en").is_err());
}
