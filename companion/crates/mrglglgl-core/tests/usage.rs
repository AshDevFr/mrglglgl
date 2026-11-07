use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mrglglgl_core::archive::{self, merge_snapshot};
use mrglglgl_core::beats::{self, Thresholds};
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::milestones;
use mrglglgl_core::schema::{self, EventKind, Journal};
use mrglglgl_core::usage::{self, Audience, DataUse};

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

fn shared_only(items: &[&str]) -> DataUse {
    DataUse {
        shared: set(items),
        view: BTreeSet::new(),
    }
}

#[test]
fn milestones_from_set_aside_data_are_hidden_and_come_back() {
    let (db, id, journal) = stored();
    let all = milestones::compute(&journal);
    assert!(all.iter().any(|a| a.id == "kept_the_boot"), "{all:?}");
    db.set_data_use(id, &shared_only(&["fishing"])).unwrap();
    let hidden = usage::awards(&db, id, Audience::Shared).unwrap();
    assert!(!hidden.iter().any(|a| a.id == "kept_the_boot"));
    assert!(hidden.len() < all.len());
    // Kept from sharing only: the window still shows the stamp.
    assert_eq!(
        usage::awards(&db, id, Audience::View).unwrap().len(),
        all.len()
    );
    db.set_data_use(id, &DataUse::default()).unwrap();
    assert_eq!(
        usage::awards(&db, id, Audience::Shared).unwrap().len(),
        all.len()
    );
}

#[test]
fn each_audience_gets_its_own_journal_and_the_archive_keeps_everything() {
    let (db, id, journal) = stored();
    db.set_data_use(
        id,
        &DataUse {
            shared: set(&["chat"]),
            view: set(&["money"]),
        },
    )
    .unwrap();
    let day = |audience| {
        archive::load_for(&db, id, audience).unwrap().unwrap().days["2026-11-08"].clone()
    };
    let shared = day(Audience::Shared);
    assert!(shared.chat.is_empty() && !shared.money.is_empty());
    let view = day(Audience::View);
    assert!(!view.chat.is_empty() && view.money.is_empty());
    let whole = archive::load(&db, id).unwrap().unwrap();
    assert_eq!(whole.days["2026-11-08"], journal.days["2026-11-08"]);
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
    let choice = db.data_use(id).unwrap();
    assert_eq!(choice.shared.len(), usage::CATEGORIES.len());
    assert_eq!(choice.view.len(), usage::CATEGORIES.len());
}
