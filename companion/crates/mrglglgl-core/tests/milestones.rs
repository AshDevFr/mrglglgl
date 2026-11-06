use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrglglgl_core::archive::merge_snapshot;
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::milestones::{self, Award, MILESTONES, compute, facts};
use mrglglgl_core::schema::{self, Day, Event, EventKind, Journal, Killer};

fn synthetic() -> Journal {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    schema::journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn awarded(awards: &[Award]) -> BTreeMap<&str, &str> {
    awards
        .iter()
        .map(|a| (a.id.as_str(), a.day.as_str()))
        .collect()
}

fn event(kind: EventKind) -> Event {
    Event {
        time: 0,
        zone: Some("Mulgore".into()),
        subzone: None,
        map_id: None,
        level: Some(5),
        kind,
    }
}

fn raw_day(events: Vec<EventKind>) -> Day {
    Day {
        events: Some(events.into_iter().map(event).collect()),
        ..Day::default()
    }
}

fn boot() -> EventKind {
    EventKind::Fish {
        item_id: Some(6150),
        name: Some("Old Boot".into()),
        junk: Some(true),
        boot: Some(true),
    }
}

#[test]
fn awards_from_raw_days_and_from_compacted_digests() {
    let awards = compute(&synthetic());
    assert_eq!(
        awarded(&awards),
        BTreeMap::from([
            // From the compacted 2026-11-01 day: only its digest remains.
            ("kept_the_boot", "2026-11-01"),
            ("sleeps_with_fishes", "2026-11-01"),
            // From raw events.
            ("first_murloc", "2026-11-08"),
            ("slain_by_murloc", "2026-11-08"),
            ("only_fished", "2026-11-09"),
            ("quiet_day", "2026-11-10"),
        ])
    );
}

#[test]
fn kept_the_boot_needs_only_one_boot_and_is_the_first_milestone() {
    let mut journal = Journal::default();
    journal
        .days
        .insert("2026-11-03".into(), raw_day(vec![boot()]));
    let awards = compute(&journal);
    assert_eq!(
        awards[0],
        Award {
            id: "kept_the_boot".into(),
            day: "2026-11-03".into()
        }
    );
    assert_eq!(MILESTONES[0].id, "kept_the_boot");
    assert!(!MILESTONES[0].proposed);
}

#[test]
fn running_totals_award_on_the_day_they_reach_the_threshold() {
    let mut journal = Journal::default();
    for day in 1..=6 {
        journal
            .days
            .insert(format!("2026-11-{day:02}"), raw_day(vec![boot()]));
    }
    let mut murlocs = raw_day(vec![]);
    murlocs.kills.insert(
        732,
        schema::KillAggregate {
            count: 60,
            name: Some("Murloc Streamrunner".into()),
            classification: "normal".into(),
            murloc: true,
        },
    );
    journal.days.insert("2026-11-07".into(), murlocs.clone());
    journal.days.insert("2026-11-08".into(), murlocs);

    let got = compute(&journal);
    let got = awarded(&got);
    assert_eq!(got["boot_collector"], "2026-11-05");
    assert_eq!(got["traitor"], "2026-11-08");
    assert_eq!(got["first_murloc"], "2026-11-07");
}

#[test]
fn a_bad_day_and_a_tourist_day() {
    let death = || EventKind::Death {
        killer: Some(Killer {
            name: Some("Prairie Wolf".into()),
            npc_id: Some(2958),
            murloc: false,
        }),
        environment: None,
    };
    let discover = |area: &str| EventKind::Discover {
        area: Some(area.into()),
        source: Some("visit".into()),
    };
    let mut journal = Journal::default();
    journal
        .days
        .insert("2026-11-01".into(), raw_day(vec![death(), death()]));
    journal.days.insert(
        "2026-11-02".into(),
        raw_day(vec![death(), death(), death()]),
    );
    journal.days.insert(
        "2026-11-03".into(),
        raw_day(["A", "B", "C", "D", "E"].map(discover).to_vec()),
    );
    let got = compute(&journal);
    let got = awarded(&got);
    assert_eq!(got["bad_day"], "2026-11-02");
    assert_eq!(got["tourist"], "2026-11-03");
    assert!(!got.contains_key("quiet_day"), "busy days are not quiet");
}

#[test]
fn facts_read_a_compacted_day_from_its_digest() {
    let journal = synthetic();
    let compacted = &journal.days["2026-11-01"];
    // The game kept the day's key moments; its facts still come from the digest.
    assert!(compacted.compacted);
    let f = facts(compacted);
    assert_eq!((f.boots, f.drownings, f.deaths, f.fish), (1, 1, 1, 2));
}

#[test]
fn awards_are_stored_once_and_never_move() {
    let db = Database::open_in_memory().unwrap();
    let id = db
        .upsert_character(
            &CharacterSource {
                flavor: "_forever_".into(),
                account: "ACCOUNT1".into(),
                realm_folder: "93".into(),
                character_folder: "Wren-Hollowbrook".into(),
                journal_path: Path::new("/wow/Mrglglgl.lua").to_path_buf(),
            },
            None,
            None,
        )
        .unwrap();

    let journal = synthetic();
    merge_snapshot(&db, id, &journal, 100).unwrap();
    let first = milestones::award(&db, id, &journal).unwrap();
    assert_eq!(awarded(&first)["kept_the_boot"], "2026-11-01");

    // The same snapshot again changes nothing.
    assert_eq!(milestones::award(&db, id, &journal).unwrap(), first);

    // An earlier boot learnt about later does not move the stamp.
    let mut earlier = journal.clone();
    earlier
        .days
        .insert("2026-10-30".into(), raw_day(vec![boot()]));
    let again = milestones::award(&db, id, &earlier).unwrap();
    assert_eq!(awarded(&again)["kept_the_boot"], "2026-11-01");
    assert_eq!(milestones::awards(&db, id).unwrap(), again);
}

#[test]
fn proposals_are_marked_and_each_can_be_turned_off() {
    let proposed: Vec<_> = MILESTONES
        .iter()
        .filter(|m| m.proposed)
        .map(|m| m.id)
        .collect();
    assert_eq!(
        proposed,
        [
            "boot_collector",
            "sleeps_with_fishes",
            "bad_day",
            "tourist",
            "quiet_day"
        ]
    );
    assert!(MILESTONES.iter().all(|m| m.enabled));
}
