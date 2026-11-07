use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mrglglgl_core::archive::{self, merge_snapshot};
use mrglglgl_core::beats::{self, Thresholds};
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::milestones;
use mrglglgl_core::schema::{self, EventKind, Journal};
use mrglglgl_core::usage;

fn synthetic() -> Journal {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    schema::journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn stored() -> (Database, i64, Journal) {
    let db = Database::open_in_memory().unwrap();
    let source = CharacterSource {
        flavor: "_forever_".into(),
        account: "ACCOUNT1".into(),
        realm_folder: "93".into(),
        character_folder: "Wren-Hollowbrook".into(),
        journal_path: Path::new("/wow/Mrglglgl.lua").into(),
    };
    let id = db.upsert_character(&source, None, None).unwrap();
    let journal = synthetic();
    merge_snapshot(&db, id, &journal, 100).unwrap();
    milestones::award(&db, id, &journal).unwrap();
    (db, id, journal)
}

#[test]
fn everything_is_used_until_the_player_sets_something_aside() {
    let j = synthetic();
    assert_eq!(usage::strip(&j, &BTreeSet::new()), j);
}

#[test]
fn a_set_aside_category_leaves_no_trace_in_events_maps_digests_or_totals() {
    let j = synthetic();
    let s = usage::strip(&j, &set(&["fishing", "money", "chat", "group"]));
    for day in s.days.values() {
        for e in day.events.iter().flatten() {
            assert!(!matches!(
                e.kind,
                EventKind::Fish { .. } | EventKind::Auction { .. } | EventKind::GroupJoin { .. }
            ));
            assert_eq!(e.balance, None);
        }
        assert!(day.money.is_empty() && day.chat.is_empty());
        if let Some(d) = &day.digest {
            assert!(d.fish.is_empty() && d.auctions.is_empty() && d.companions.is_empty());
            assert_eq!((d.counts.fish, d.counts.money_in, d.counts.chat), (0, 0, 0));
            assert_eq!((d.bal_start, d.bal_end), (None, None));
        }
    }
    assert_eq!(s.stats.boots, 0);
    assert!(s.companions.is_empty());
    // Untouched categories stay.
    assert_eq!(s.quests, j.quests);
    assert_eq!(s.days["2026-11-08"].kills, j.days["2026-11-08"].kills);
}

#[test]
fn the_narrator_never_hears_of_set_aside_data() {
    let j = usage::strip(&synthetic(), &set(&["money", "chat"]));
    let all = Thresholds {
        max_beats: 20,
        ..Thresholds::default()
    };
    let texts = beats::extract(&j, "2026-11-08", &[], &all).texts();
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("auction") || t.contains("Earned") || t.contains("In character")),
        "{texts:?}"
    );
}

#[test]
fn milestones_from_set_aside_data_are_hidden_and_come_back() {
    let (db, id, journal) = stored();
    let all = milestones::compute(&journal);
    assert!(all.iter().any(|a| a.id == "kept_the_boot"), "{all:?}");
    db.set_unused_data(id, &set(&["fishing"])).unwrap();
    let hidden = usage::awards(&db, id).unwrap();
    assert!(!hidden.iter().any(|a| a.id == "kept_the_boot"));
    assert!(hidden.len() < all.len());
    db.set_unused_data(id, &BTreeSet::new()).unwrap();
    assert_eq!(usage::awards(&db, id).unwrap().len(), all.len());
}

#[test]
fn the_archive_keeps_everything_and_the_window_gets_what_is_in_use() {
    let (db, id, journal) = stored();
    db.set_unused_data(id, &set(&["chat"])).unwrap();
    assert_eq!(db.unused_data(id).unwrap(), set(&["chat"]));
    let in_use = archive::load_in_use(&db, id).unwrap().unwrap();
    assert!(in_use.days["2026-11-08"].chat.is_empty());
    let whole = archive::load(&db, id).unwrap().unwrap();
    assert_eq!(
        whole.days["2026-11-08"].chat,
        journal.days["2026-11-08"].chat
    );
}

#[test]
fn an_unreadable_choice_shares_nothing_rather_than_everything() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let source = CharacterSource {
        flavor: "_forever_".into(),
        account: "ACCOUNT1".into(),
        realm_folder: "93".into(),
        character_folder: "Wren-Hollowbrook".into(),
        journal_path: Path::new("/wow/Mrglglgl.lua").into(),
    };
    let id = db.upsert_character(&source, None, None).unwrap();
    rusqlite::Connection::open(dir.path().join("mrglglgl.db"))
        .unwrap()
        .execute(
            "UPDATE characters SET unused_data = 'not json' WHERE id = ?1",
            [id],
        )
        .unwrap();
    assert_eq!(db.unused_data(id).unwrap().len(), usage::CATEGORIES.len());
}
