use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mrglglgl_core::continuity::{self, ContinuitySheet, MAX_JOKES, Nemesis, Recurring};
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::milestones::Award;
use mrglglgl_core::schema::{Day, Event, EventKind, Journal, Killer, Stats};

fn event(kind: EventKind) -> Event {
    Event {
        time: 1_794_160_000,
        zone: Some("Mulgore".into()),
        subzone: None,
        map_id: None,
        level: Some(9),
        balance: None,
        kind,
    }
}

fn death_to(name: &str, murloc: bool) -> Event {
    event(EventKind::Death {
        killer: Some(Killer {
            name: Some(name.into()),
            npc_id: None,
            murloc,
        }),
        environment: None,
    })
}

fn boot() -> Event {
    event(EventKind::Fish {
        item_id: Some(6150),
        name: Some("Old Boot".into()),
        junk: Some(true),
        boot: Some(true),
    })
}

fn met(who: &str) -> Event {
    event(EventKind::GroupJoin {
        who: Some(who.into()),
    })
}

fn day(events: Vec<Event>) -> Day {
    Day {
        events: Some(events),
        ..Day::default()
    }
}

/// Three days: a boot and two murloc deaths, then a companion twice, then another boot.
fn journal() -> Journal {
    let mut j = Journal {
        schema: 1,
        stats: Stats {
            boots: 3,
            deaths_by_murloc: 2,
            ..Stats::default()
        },
        ..Journal::default()
    };
    j.days.insert(
        "2026-11-15".into(),
        day(vec![
            boot(),
            death_to("Murloc Streamrunner", true),
            death_to("Murloc Streamrunner", true),
            met("Rowan-Elm"),
        ]),
    );
    j.days.insert(
        "2026-11-16".into(),
        day(vec![met("Rowan-Elm"), met("Tamsin-Reedwater")]),
    );
    j.days.insert("2026-11-17".into(), day(vec![boot()]));
    j
}

fn awards() -> Vec<Award> {
    vec![Award {
        id: "kept_the_boot".into(),
        day: "2026-11-15".into(),
        rarity: 1,
    }]
}

fn database_with_character() -> (Database, i64) {
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
    (db, id)
}

#[test]
fn facts_come_from_the_journal() {
    let sheet = ContinuitySheet::compute(&journal(), &awards(), &[], None);
    assert_eq!(sheet.boots, 3);
    assert_eq!(sheet.deaths_by_murloc, 2);
    assert_eq!(
        sheet.nemesis,
        Some(Nemesis {
            name: "Murloc Streamrunner".into(),
            deaths: 2
        })
    );
    assert_eq!(
        sheet.companions,
        vec![Recurring {
            name: "Rowan".into(),
            days: 2
        }]
    );
    assert_eq!(sheet.titles, vec!["Kept the Boot"]);
}

#[test]
fn an_old_day_remembers_the_totals_as_they_were() {
    // The journal counts three boots, one of them from before the archive began. At the start of
    // the 16th the boot fished on the 17th had not happened yet.
    let sheet = ContinuitySheet::compute(&journal(), &awards(), &[], Some("2026-11-16"));
    assert_eq!(sheet.boots, 2);
    assert_eq!(sheet.deaths_by_murloc, 2);
    assert!(
        sheet.companions.is_empty(),
        "Rowan was met only once by then"
    );
    let before_anything = ContinuitySheet::compute(&journal(), &awards(), &[], Some("2026-11-15"));
    assert!(before_anything.titles.is_empty());
    assert_eq!(before_anything.nemesis, None);
}

#[test]
fn the_boot_count_is_the_journals_whatever_the_model_says() {
    let (db, id) = database_with_character();
    let sheet = continuity::refresh(
        &db,
        id,
        &journal(),
        &awards(),
        Some(r#"{"runningJokes": ["The pond owes Wren a fish"], "boots": 99}"#),
    )
    .unwrap();
    assert_eq!(sheet.boots, 3);
    assert_eq!(sheet.running_jokes, vec!["The pond owes Wren a fish"]);
    assert_eq!(continuity::load(&db, id).unwrap(), Some(sheet));
}

/// Collects what is logged while `f` runs.
fn logs_of(f: impl FnOnce()) -> String {
    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);
    impl Write for Buffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap()
}

#[test]
fn malformed_model_json_leaves_the_sheet_unchanged_and_warns() {
    let (db, id) = database_with_character();
    let before = continuity::refresh(
        &db,
        id,
        &journal(),
        &awards(),
        Some(r#"{"runningJokes": ["Boots are a currency"]}"#),
    )
    .unwrap();
    let mut after = None;
    let logs = logs_of(|| {
        after = Some(
            continuity::refresh(&db, id, &journal(), &awards(), Some("{jokes: oops")).unwrap(),
        );
    });
    assert_eq!(after.unwrap(), before);
    assert!(logs.contains("WARN"), "{logs}");
    assert!(logs.contains("sheet left unchanged"), "{logs}");
}

#[test]
fn running_jokes_are_validated_and_capped() {
    let sheet = ContinuitySheet::compute(&journal(), &awards(), &[], None);
    let long = "x".repeat(200);
    let json = serde_json::json!({
        "running_jokes": ["one", "", "  two  ", "ONE", long, "three", "four", "five", "six"]
    })
    .to_string();
    let updated = sheet.with_model_update(&json).unwrap();
    assert_eq!(updated.running_jokes.len(), MAX_JOKES);
    assert_eq!(
        updated.running_jokes,
        vec!["two", "three", "four", "five", "six"]
    );
    assert!(
        sheet
            .with_model_update(r#"{"running_jokes": "not a list"}"#)
            .is_err()
    );
}

#[test]
fn memory_lines_leave_out_what_is_empty() {
    let empty = ContinuitySheet::default();
    assert!(empty.memory_lines().is_empty());
    let sheet = ContinuitySheet::compute(&journal(), &awards(), &[], None);
    assert_eq!(
        sheet.memory_lines(),
        vec![
            "Boots kept so far: 3",
            "Deaths by murloc in total: 2",
            "Recurring companion: Rowan (2 days together)",
            "Nemesis: Murloc Streamrunner (died to them 2 times)",
            "Milestones earned: Kept the Boot",
        ]
    );
}
