use std::path::PathBuf;

use mrglglgl_core::archive;
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::queue::{self, Ask};
use mrglglgl_core::review;
use mrglglgl_core::schema::{Character, Day, Journal};
use mrglglgl_core::stories::{self, NewStory, StoryKind, StoryStatus};
use mrglglgl_core::writer::CheckStatus;

/// Monday 2026-11-02 to Wednesday 2026-11-04 played, and today, Monday 2026-11-09.
const DAYS: [&str; 4] = ["2026-11-02", "2026-11-03", "2026-11-04", "2026-11-09"];
const TODAY: &str = "2026-11-09";

fn setup() -> (Database, i64, Journal) {
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
    let mut journal = Journal {
        schema: 1,
        character: Character {
            name: Some("Wren".into()),
            ..Character::default()
        },
        ..Journal::default()
    };
    for day in DAYS {
        journal.days.insert(day.into(), Day::default());
    }
    archive::merge_snapshot(&db, id, &journal, 1).unwrap();
    (db, id, journal)
}

fn save(db: &Database, id: i64, kind: StoryKind, day: &str, status: StoryStatus) -> i64 {
    stories::save(
        db,
        &NewStory {
            character_id: id,
            kind,
            start_day: day.into(),
            end_day: day.into(),
            narrator: "murloc_scribe".into(),
            language: "en".into(),
            provider: "fake".into(),
            model: "fake".into(),
            prompt_hash: "h".into(),
            text: format!("{status:?} {day}"),
            check: CheckStatus::Passed,
            check_reason: None,
            status,
            note: None,
        },
    )
    .unwrap()
}

#[test]
fn lists_finished_days_and_weeks_without_an_accepted_entry_newest_first() {
    let (db, id, journal) = setup();
    save(&db, id, StoryKind::Day, DAYS[0], StoryStatus::Accepted);
    let draft = save(&db, id, StoryKind::Day, DAYS[1], StoryStatus::Draft);
    save(&db, id, StoryKind::Day, DAYS[2], StoryStatus::Discarded);
    queue::request(&db, id, StoryKind::Week, "2026-11-02", &Ask::default()).unwrap();

    let items = review::items(&db, id, &journal, TODAY).unwrap();
    let slots: Vec<(StoryKind, &str)> = items
        .iter()
        .map(|i| (i.kind, i.start_day.as_str()))
        .collect();
    assert_eq!(
        slots,
        [
            (StoryKind::Week, "2026-11-02"),
            (StoryKind::Day, DAYS[2]),
            (StoryKind::Day, DAYS[1]),
        ],
        "accepted days and today are left out; the finished week is in"
    );
    let week = &items[0];
    assert_eq!(week.end_day, "2026-11-08");
    assert!(week.job.is_some(), "the week is being written");
    assert!(items[1].discarded && items[1].drafts.is_empty());
    assert_eq!(items[2].drafts.len(), 1);
    assert_eq!(items[2].drafts[0].id, draft);
    assert!(!items[2].discarded);

    assert_eq!(
        review::waiting(&db, id).unwrap(),
        1,
        "only drafts wait for the player"
    );
    assert!(stories::accept(&db, id, draft).unwrap());
    let items = review::items(&db, id, &journal, TODAY).unwrap();
    assert!(items.iter().all(|i| i.start_day != DAYS[1]));
    assert_eq!(review::waiting(&db, id).unwrap(), 0);
}

#[test]
fn a_day_with_an_accepted_entry_and_a_new_draft_is_listed_for_its_draft() {
    let (db, id, journal) = setup();
    save(&db, id, StoryKind::Day, DAYS[0], StoryStatus::Accepted);
    save(&db, id, StoryKind::Day, DAYS[0], StoryStatus::Draft);
    let items = review::items(&db, id, &journal, TODAY).unwrap();
    let day = items
        .iter()
        .find(|i| i.kind == StoryKind::Day && i.start_day == DAYS[0])
        .unwrap();
    assert_eq!(day.drafts.len(), 1);
    assert!(
        day.has_entry,
        "the journal keeps its entry until the draft is accepted"
    );
}
