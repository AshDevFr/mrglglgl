use std::path::PathBuf;
use std::sync::Mutex;

use mrglglgl_core::archive;
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::gists;
use mrglglgl_core::queue::{self, Ask, Control, run_next};
use mrglglgl_core::schema::{Character, Day, Event, EventKind, Journal, Quest};
use mrglglgl_core::stories::StoryKind;
use mrglglgl_llm::fake::FakeProvider;

const DAY: &str = "2026-11-02";
const ENTRY: &str = "A long day of errands, and the Barrens were kind.";

fn turnin(quest: i64) -> Event {
    Event {
        time: 1_794_160_000,
        zone: Some("The Barrens".into()),
        subzone: None,
        map_id: None,
        level: Some(9),
        balance: None,
        kind: EventKind::QuestTurnin {
            quest_id: Some(quest),
            xp: Some(100),
            money: Some(10),
        },
    }
}

fn quest(title: &str, text: Option<&str>, last: &str) -> Quest {
    Quest {
        title: Some(title.into()),
        text: text.map(str::to_string),
        done: None,
        last: Some(last.into()),
        ..Quest::default()
    }
}

/// A day handing in a quest with text and one without; an older quest from an earlier day.
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
            events: Some(vec![turnin(1), turnin(2)]),
            ..Day::default()
        },
    );
    j.days.insert(
        "2026-11-01".into(),
        Day {
            events: Some(vec![turnin(3)]),
            ..Day::default()
        },
    );
    j.quests.insert(
        1,
        quest(
            "Sample for Helbrim",
            Some("Apothecary Helbrim at the Crossroads would like a sample of the wolf hearts."),
            DAY,
        ),
    );
    j.quests.insert(2, quest("Plainstrider Menace", None, DAY));
    j.quests.insert(
        3,
        quest(
            "Raptor Thieves",
            Some("Raptors have been stealing from the Crossroads. Bring back their heads."),
            "2026-11-01",
        ),
    );
    j
}

fn setup() -> (Mutex<Database>, i64) {
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
    (Mutex::new(db), id)
}

#[tokio::test]
async fn a_days_quests_are_summed_up_before_its_entry_and_the_entry_is_told_with_them() {
    let (db, id) = setup();
    queue::request(
        &db.lock().unwrap(),
        id,
        StoryKind::Day,
        DAY,
        &Ask::default(),
    )
    .unwrap();
    let gist = "Apothecary Helbrim at the Crossroads wanted a sample of wolf hearts.";
    let provider = FakeProvider::new([gist, ENTRY]);
    run_next(&db, &provider, &Control::new(false), "en", &|_| {})
        .await
        .unwrap();
    let requests = provider.requests();
    assert_eq!(
        requests.len(),
        2,
        "one gist, then the entry: the quest without text has none"
    );
    assert!(requests[0].user.contains("wolf hearts"));
    assert!(
        requests[1]
            .user
            .contains(&format!("- Sample for Helbrim: {gist}")),
        "{}",
        requests[1].user
    );
    assert!(
        !requests[1].user.contains("would like a sample"),
        "the quest's own text is not sent with the entry"
    );
    assert!(requests[1].user.contains("- Plainstrider Menace\n"));
    assert_eq!(
        gists::load(&db.lock().unwrap(), "en").unwrap().get(&1),
        Some(&gist.to_string())
    );
}

#[tokio::test]
async fn a_gist_that_names_what_the_quest_does_not_is_dropped_and_not_asked_again() {
    let (db, id) = setup();
    let invented = "Thrall himself asked for wolf hearts in Orgrimmar.";
    let provider = FakeProvider::new([invented]);
    let control = Control::new(false);
    let written = gists::for_day(&db, &provider, &control, id, DAY, "en")
        .await
        .unwrap();
    assert_eq!(written, 0);
    assert!(gists::load(&db.lock().unwrap(), "en").unwrap().is_empty());
    let again = FakeProvider::new(Vec::<String>::new());
    gists::for_day(&db, &again, &control, id, DAY, "en")
        .await
        .unwrap();
    assert!(
        again.requests().is_empty(),
        "the same text is not tried twice"
    );
}

#[tokio::test]
async fn idle_time_sums_up_the_newest_quests_first_a_few_at_a_time() {
    let (db, _) = setup();
    let provider = FakeProvider::new([
        "Apothecary Helbrim at the Crossroads wanted wolf hearts.",
        "Raptors stole from the Crossroads, and their heads were wanted.",
    ]);
    let control = Control::new(false);
    let written = gists::backfill(&db, &provider, &control, "en", 1)
        .await
        .unwrap();
    assert_eq!(written, 1);
    assert!(provider.requests()[0].user.contains("Sample for Helbrim"));
    gists::backfill(&db, &provider, &control, "en", 5)
        .await
        .unwrap();
    assert_eq!(gists::load(&db.lock().unwrap(), "en").unwrap().len(), 2);
    assert_eq!(
        gists::backfill(&db, &provider, &control, "en", 5)
            .await
            .unwrap(),
        0,
        "nothing left to sum up"
    );
}

#[tokio::test]
async fn nothing_is_summed_up_while_writing_is_paused() {
    let (db, _) = setup();
    let provider = FakeProvider::new(Vec::<String>::new());
    let written = gists::backfill(&db, &provider, &Control::new(true), "en", 5)
        .await
        .unwrap();
    assert_eq!(written, 0);
    assert!(provider.requests().is_empty());
}
