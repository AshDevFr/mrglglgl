use std::path::PathBuf;
use std::sync::Mutex;

use mrglglgl_core::archive;
use mrglglgl_core::db::{Database, Planning};
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::planning::{self, Plan};
use mrglglgl_core::queue::{self, Ask, Control, run_next};
use mrglglgl_core::schema::{Character, Day, Event, EventKind, Journal, Killer};
use mrglglgl_core::stories::StoryKind;
use mrglglgl_llm::fake::FakeProvider;

const DAY: &str = "2026-11-02";
const ENTRY: &str = "A fall, a fish, and the long walk home.";

fn event(kind: EventKind) -> Event {
    Event {
        time: 1_794_160_000,
        zone: Some("Mulgore".into()),
        subzone: Some("Red Rocks".into()),
        map_id: None,
        level: Some(9),
        balance: None,
        kind,
    }
}

/// A day with a death, a level and some fishing.
fn journal() -> Journal {
    let mut j = Journal {
        schema: 1,
        character: Character {
            name: Some("Wren".into()),
            ..Character::default()
        },
        ..Journal::default()
    };
    j.days.insert(
        DAY.into(),
        Day {
            events: Some(vec![
                event(EventKind::Death {
                    final_death: false,
                    killer: Some(Killer {
                        name: Some("Prairie Wolf".into()),
                        npc_id: Some(2958),
                        murloc: false,
                    }),
                    environment: None,
                }),
                event(EventKind::Level),
                event(EventKind::Fish {
                    item_id: Some(6291),
                    name: Some("Raw Brilliant Smallfish".into()),
                    junk: Some(false),
                    boot: Some(false),
                }),
            ]),
            ..Day::default()
        },
    );
    j
}

fn setup(planning: Planning) -> (Mutex<Database>, i64) {
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
    let mut settings = db.settings().unwrap();
    settings.writing.planning = planning;
    db.save_settings(&settings).unwrap();
    queue::request(&db, id, StoryKind::Day, DAY, &Ask::default()).unwrap();
    (Mutex::new(db), id)
}

#[test]
fn a_plan_is_the_moments_chosen_in_order_and_the_thread_between_them() {
    let notes = 5;
    assert_eq!(
        planning::parse("Moments: 3, 1\nArc: a hard fall, then the water.", notes),
        Some(Plan {
            moments: vec![2, 0],
            arc: Some("a hard fall, then the water.".into()),
        })
    );
    // Numbers that name no note, or name one twice, are dropped; at most four are kept.
    assert_eq!(
        planning::parse("Moments: 9, 2, 2, 1, 3, 4, 5", notes).map(|p| p.moments),
        Some(vec![1, 0, 2, 3])
    );
    assert_eq!(planning::parse("I think the day was nice.", notes), None);
    assert_eq!(planning::parse("Moments: 7", notes), None);
}

#[test]
fn automatic_planning_is_for_the_models_on_this_computer() {
    assert!(Planning::Auto.applies("local"));
    assert!(!Planning::Auto.applies("endpoint"));
    assert!(Planning::On.applies("endpoint"));
    assert!(!Planning::Off.applies("local"));
}

#[tokio::test]
async fn a_planned_day_tells_the_chosen_moments_first_in_the_planned_order() {
    let (db, _) = setup(Planning::On);
    let provider = FakeProvider::new(["Moments: 3, 4\nArc: a fall, then the quiet water.", ENTRY]);
    run_next(&db, &provider, &Control::new(false), "en", &|_| {})
        .await
        .unwrap();
    let requests = provider.requests();
    assert_eq!(requests.len(), 2, "the plan, then the entry");
    let plan = &requests[0].user;
    assert!(plan.contains("3. Went fishing"), "{plan}");
    assert!(plan.contains("4. Died to a Prairie Wolf"), "{plan}");
    let entry = &requests[1].user;
    let at = |text: &str| {
        entry
            .find(text)
            .unwrap_or_else(|| panic!("{text} in {entry}"))
    };
    assert!(at("The heart of the day:") < at("Went fishing"));
    assert!(at("Went fishing") < at("Died to a Prairie Wolf"));
    assert!(at("Died to a Prairie Wolf") < at("In passing, at most:"));
    assert!(entry.contains("The thread of the day: a fall, then the quiet water."));
}

#[tokio::test]
async fn an_arc_naming_what_the_notes_do_not_is_left_out_and_a_failed_plan_changes_nothing() {
    let (db, _) = setup(Planning::On);
    let provider = FakeProvider::new(["Moments: 1\nArc: Thrall watched from Orgrimmar.", ENTRY]);
    run_next(&db, &provider, &Control::new(false), "en", &|_| {})
        .await
        .unwrap();
    assert!(!provider.requests()[1].user.contains("Thrall"));

    let (db, _) = setup(Planning::On);
    let provider = FakeProvider::new(["The day was lovely.", ENTRY]);
    run_next(&db, &provider, &Control::new(false), "en", &|_| {})
        .await
        .unwrap();
    assert_eq!(provider.requests().len(), 2);
    assert!(
        !provider.requests()[1]
            .user
            .contains("The thread of the day")
    );
}

#[tokio::test]
async fn with_planning_off_the_entry_is_the_only_request() {
    let (db, _) = setup(Planning::Off);
    let provider = FakeProvider::new([ENTRY]);
    run_next(&db, &provider, &Control::new(false), "en", &|_| {})
        .await
        .unwrap();
    assert_eq!(provider.requests().len(), 1);
}
