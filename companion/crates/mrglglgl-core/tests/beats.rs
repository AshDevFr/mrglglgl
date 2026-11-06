use std::collections::BTreeMap;
use std::path::PathBuf;

use mrglglgl_core::beats::{self, Beat, DayRecord, Pattern, Thresholds};
use mrglglgl_core::milestones::Award;
use mrglglgl_core::schema::{
    Day, Digest, DigestCounts, DigestDeath, Event, EventKind, Journal, KillAggregate, Killer,
    Quest, SkillRecord, journal_from_text,
};

fn event(kind: EventKind) -> Event {
    Event {
        time: 1_794_160_000,
        zone: Some("Mulgore".into()),
        subzone: Some("Red Rocks".into()),
        map_id: Some(7),
        level: Some(9),
        kind,
    }
}

fn kills(list: &[(i64, &str, i64, &str, bool)]) -> BTreeMap<i64, KillAggregate> {
    list.iter()
        .map(|(npc, name, count, cls, murloc)| {
            (
                *npc,
                KillAggregate {
                    count: *count,
                    name: Some(name.to_string()),
                    classification: cls.to_string(),
                    murloc: *murloc,
                },
            )
        })
        .collect()
}

fn boot() -> EventKind {
    EventKind::Fish {
        item_id: Some(6150),
        name: Some("Old Boot".into()),
        junk: Some(true),
        boot: Some(true),
    }
}

fn murloc_death() -> EventKind {
    EventKind::Death {
        killer: Some(Killer {
            name: Some("Murloc Streamrunner".into()),
            npc_id: Some(732),
            murloc: true,
        }),
        environment: None,
    }
}

/// The day the spec describes: 23 Prowlers, one boot, one death by murloc.
fn prowler_day() -> Day {
    Day {
        events: Some(vec![event(boot()), event(murloc_death())]),
        kills: kills(&[(3425, "Prowler", 23, "normal", false)]),
        ..Day::default()
    }
}

fn journal(days: Vec<(&str, Day)>) -> Journal {
    Journal {
        schema: 1,
        days: days
            .into_iter()
            .map(|(d, day)| (d.to_string(), day))
            .collect(),
        ..Journal::default()
    }
}

fn award(id: &str, day: &str) -> Award {
    Award {
        id: id.into(),
        day: day.into(),
    }
}

#[test]
fn thresholds_come_from_the_data_file() {
    let t = Thresholds::default();
    assert_eq!(t.max_beats, 9);
    assert_eq!(t.many_deaths, 3);
}

#[test]
fn the_same_day_always_gives_the_same_beats() {
    let j = journal(vec![("2026-11-17", prowler_day())]);
    let awards = [award("kept_the_boot", "2026-11-17")];
    let first = beats::extract(&j, "2026-11-17", &awards, &Thresholds::default());
    let second = beats::extract(&j, "2026-11-17", &awards, &Thresholds::default());
    assert_eq!(first, second);
    assert_eq!(first.texts(), second.texts());
}

#[test]
fn a_prowler_day_keeps_the_boot_and_the_murloc_death() {
    let j = journal(vec![("2026-11-17", prowler_day())]);
    let day = beats::extract(&j, "2026-11-17", &[], &Thresholds::default());
    assert!(day.beats.len() <= 8);
    let kill_beats: Vec<&Beat> = day
        .beats
        .iter()
        .filter(|b| matches!(b, Beat::Kills { .. }))
        .collect();
    assert_eq!(kill_beats.len(), 1);
    assert_eq!(kill_beats[0].text(), "Killed Prowler ×23");
    let texts = day.texts();
    assert!(texts.contains(&"Fished up an Old Boot and kept it".to_string()));
    assert!(texts.contains(&"Died to a Murloc Streamrunner in Red Rocks".to_string()));
    assert!(day.patterns.contains(&Pattern::MurlocDeath));
}

#[test]
fn a_crowded_day_is_cut_to_nine_beats_keeping_the_important_ones() {
    let mut day = prowler_day();
    let events = day.events.as_mut().unwrap();
    events.push(event(EventKind::Level));
    events.push(Event {
        level: Some(10),
        ..event(EventKind::Level)
    });
    events.push(event(EventKind::Loot {
        item_id: Some(1),
        name: Some("Gnarled Staff".into()),
        quality: Some(3),
        quantity: None,
    }));
    events.push(event(EventKind::GroupJoin {
        who: Some("Rowan-Elm".into()),
    }));
    events.push(event(EventKind::Boss {
        encounter_id: Some(1),
        name: Some("Lady Tidesong".into()),
    }));
    events.push(Event {
        zone: Some("The Barrens".into()),
        subzone: None,
        ..event(EventKind::Zone)
    });
    day.kills
        .extend(kills(&[(3035, "Flatland Cougar", 1, "rare", false)]));
    day.skills.insert(
        44,
        SkillRecord {
            name: Some("Axes".into()),
            category: Some("Weapon Skills".into()),
            from: 40,
            to: 43,
        },
    );
    let j = journal(vec![("2026-11-17", day)]);
    let awards = [award("kept_the_boot", "2026-11-17")];
    let day = beats::extract(&j, "2026-11-17", &awards, &Thresholds::default());

    assert_eq!(day.beats.len(), 9);
    let has = |f: fn(&Beat) -> bool| day.beats.iter().any(f);
    assert!(has(|b| matches!(b, Beat::Boot { .. })));
    assert!(has(|b| matches!(b, Beat::Death { .. })));
    assert!(has(|b| matches!(b, Beat::Milestones { .. })));
    assert!(has(|b| matches!(b, Beat::LevelUp { .. })));
    // Weapon practice is the first thing dropped from a busy day.
    assert!(!has(|b| matches!(b, Beat::Practice { .. })));
    assert!(day.patterns.contains(&Pattern::RareLoot));
    assert!(
        has(|b| matches!(b, Beat::Loot { .. })),
        "the rare find the pattern speaks of is in the notes"
    );
    assert!(day.patterns.contains(&Pattern::BigLevelJump));
    assert!(day.patterns.contains(&Pattern::NewZone));
}

#[test]
fn weapon_practice_is_told_on_a_quiet_day() {
    let mut day = Day {
        events: Some(vec![event(EventKind::SessionStart)]),
        ..Day::default()
    };
    day.skills.insert(
        44,
        SkillRecord {
            name: Some("Axes".into()),
            category: Some("Weapon Skills".into()),
            from: 40,
            to: 43,
        },
    );
    let j = journal(vec![("2026-11-17", day)]);
    let day = beats::extract(&j, "2026-11-17", &[], &Thresholds::default());
    assert!(
        day.texts()
            .contains(&"Practised Axes from 40 to 43".to_string()),
        "{:?}",
        day.texts()
    );
    assert!(!day.patterns.contains(&Pattern::NothingHappened));
}

#[test]
fn professions_crafting_and_gathering_become_beats() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    let j = journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap();
    // The fixture day is busy enough that eight beats leave professions out; the wording is
    // what is checked here.
    let all = Thresholds {
        max_beats: 20,
        ..Thresholds::default()
    };
    let texts = beats::extract(&j, "2026-11-08", &[], &all).texts();
    for expected in [
        "Mining rose from 40 to 41",
        "First Aid rose from 30 to 35",
        "Crafted Linen Bandage ×5",
        "Gathered Copper Ore ×1 and Rough Stone ×1 (Mining)",
    ] {
        assert!(
            texts.iter().any(|t| t == expected),
            "missing {expected}: {texts:?}"
        );
    }
}

#[test]
fn many_deaths_collapse_by_cause() {
    let day = Day {
        events: Some(vec![
            event(murloc_death()),
            event(murloc_death()),
            event(EventKind::Death {
                killer: None,
                environment: Some("Drowning".into()),
            }),
        ]),
        ..Day::default()
    };
    let j = journal(vec![("2026-11-17", day)]);
    let day = beats::extract(&j, "2026-11-17", &[], &Thresholds::default());
    let texts = day.texts();
    assert!(texts.contains(&"Died to a Murloc Streamrunner in Red Rocks (2 times)".to_string()));
    assert!(texts.contains(&"Drowned in Red Rocks".to_string()));
    assert!(day.patterns.contains(&Pattern::ManyDeaths));
}

#[test]
fn places_and_companions_seen_before_are_not_new() {
    let met = Day {
        events: Some(vec![event(EventKind::GroupJoin {
            who: Some("Rowan-Elm".into()),
        })]),
        ..Day::default()
    };
    let j = journal(vec![("2026-11-16", met.clone()), ("2026-11-17", met)]);
    let day = beats::extract(&j, "2026-11-17", &[], &Thresholds::default());
    assert!(!day.patterns.contains(&Pattern::NewZone));
    assert!(day.texts().contains(&"Grouped with Rowan".to_string()));
    let first = beats::extract(&j, "2026-11-16", &[], &Thresholds::default());
    assert!(first.patterns.contains(&Pattern::NewZone));
    assert!(
        first
            .texts()
            .contains(&"Grouped with Rowan (for the first time)".to_string())
    );
}

#[test]
fn a_fishing_streak_is_counted_back_over_consecutive_days() {
    let fishing = || Day {
        events: Some(vec![event(EventKind::Fish {
            item_id: Some(6291),
            name: Some("Raw Brilliant Smallfish".into()),
            junk: Some(false),
            boot: Some(false),
        })]),
        ..Day::default()
    };
    let j = journal(vec![
        ("2026-11-14", fishing()),
        ("2026-11-15", fishing()),
        ("2026-11-16", fishing()),
    ]);
    let day = beats::extract(&j, "2026-11-16", &[], &Thresholds::default());
    assert!(
        day.texts()
            .contains(&"Went fishing and caught 1 fish (day 3 of fishing in a row)".to_string()),
        "{:?}",
        day.texts()
    );
    assert!(day.patterns.contains(&Pattern::OnlyFished));
}

#[test]
fn a_compacted_day_is_read_from_its_digest() {
    let day = Day {
        events: None,
        digest: Some(Digest {
            counts: DigestCounts {
                murlocs: 12,
                deaths: 1,
                ..DigestCounts::default()
            },
            zones: vec!["Westfall".into()],
            kills: vec![mrglglgl_core::schema::DigestKill {
                npc_id: Some(126),
                name: Some("Murloc Coastrunner".into()),
                count: 12,
                classification: "normal".into(),
                murloc: true,
            }],
            deaths: vec![DigestDeath {
                killer: Some(Killer {
                    name: Some("Murloc Coastrunner".into()),
                    npc_id: Some(126),
                    murloc: true,
                }),
                environment: None,
                zone: Some("Westfall".into()),
                subzone: None,
            }],
            ..Digest::default()
        }),
        ..Day::default()
    };
    assert_eq!(DayRecord::of(&day).murloc_kills(), 12);
    // A compacted day that kept its key events (here a session start) is still told from its
    // digest, which counted everything the day held.
    let mut kept = day.clone();
    kept.compacted = true;
    kept.events = Some(vec![event(EventKind::SessionStart)]);
    assert_eq!(DayRecord::of(&kept).murloc_kills(), 12);
    assert_eq!(
        beats::extract(
            &journal(vec![("2026-11-17", kept)]),
            "2026-11-17",
            &[],
            &Thresholds::default()
        )
        .texts(),
        beats::extract(
            &journal(vec![("2026-11-17", day.clone())]),
            "2026-11-17",
            &[],
            &Thresholds::default()
        )
        .texts(),
    );
    let j = journal(vec![("2026-11-17", day)]);
    let day = beats::extract(&j, "2026-11-17", &[], &Thresholds::default());
    assert!(day.patterns.contains(&Pattern::MurlocHeavy));
    assert!(day.patterns.contains(&Pattern::MurlocDeath));
    assert_eq!(
        day.texts(),
        vec![
            "Spent the day in Westfall",
            "Visited for the first time: Westfall",
            "Killed Murloc Coastrunner ×12",
            "Died to a Murloc Coastrunner in Westfall",
        ]
    );
}

#[test]
fn a_thin_digest_is_told_from_its_exact_counts() {
    // An old day: the digest keeps highlights in its lists, and the counts stay exact.
    let day = Day {
        events: None,
        digest: Some(Digest {
            version: 2,
            thin: true,
            counts: DigestCounts {
                kills: 40,
                anon_kills: 2,
                murlocs: 15,
                deaths: 3,
                fish: 12,
                junk: 2,
                ..DigestCounts::default()
            },
            kills: vec![mrglglgl_core::schema::DigestKill {
                npc_id: Some(126),
                name: Some("Murloc Coastrunner".into()),
                count: 10,
                classification: "normal".into(),
                murloc: true,
            }],
            deaths: vec![DigestDeath {
                killer: Some(Killer {
                    name: Some("Murloc Coastrunner".into()),
                    npc_id: Some(126),
                    murloc: true,
                }),
                environment: None,
                zone: Some("Westfall".into()),
                subzone: None,
            }],
            ..Digest::default()
        }),
        ..Day::default()
    };
    let record = DayRecord::of(&day);
    assert_eq!(record.murloc_kills(), 15, "the count, not the listed 10");
    assert_eq!(record.deaths.len(), 3);
    let j = journal(vec![("2026-08-01", day)]);
    let day = beats::extract(&j, "2026-08-01", &[], &Thresholds::default());
    let texts = day.texts();
    assert!(
        texts.contains(&"Killed Murloc Coastrunner ×10 and 30 other creatures".to_string()),
        "{texts:?}"
    );
    assert!(texts.contains(&"Died to a Murloc Coastrunner in Westfall".to_string()));
    assert!(texts.contains(&"Died (2 times)".to_string()), "{texts:?}");
    assert!(
        texts.contains(&"Went fishing and caught 10 fish".to_string()),
        "{texts:?}"
    );
    assert!(day.patterns.contains(&Pattern::MurlocHeavy));
    assert!(day.patterns.contains(&Pattern::ManyDeaths));
}

#[test]
fn a_day_missing_from_the_journal_has_no_beats() {
    let j = journal(vec![]);
    assert!(
        beats::extract(&j, "2026-11-17", &[], &Thresholds::default())
            .beats
            .is_empty()
    );
}

#[test]
fn completed_and_new_quests_are_events_of_the_day() {
    let accept = |q| event(EventKind::QuestAccept { quest_id: Some(q) });
    let turnin = |q| {
        event(EventKind::QuestTurnin {
            quest_id: Some(q),
            xp: Some(450),
            money: Some(75),
        })
    };
    let day = Day {
        // Taken and finished the same day (1); finished, taken earlier (2); finished with no
        // title known (3, taken before the addon); taken and still open (4).
        events: Some(vec![accept(1), turnin(1), turnin(2), turnin(3), accept(4)]),
        ..Day::default()
    };
    let mut j = journal(vec![("2026-11-17", day)]);
    for (id, title) in [
        (1, "The Ravaged Caravan"),
        (2, "Wildmane Totem"),
        (4, "Swoop Hunting"),
    ] {
        j.quests.insert(
            id,
            Quest {
                title: Some(title.into()),
                ..Quest::default()
            },
        );
    }
    let texts = beats::extract(&j, "2026-11-17", &[], &Thresholds::default()).texts();
    assert!(
        texts.contains(
            &"Completed the quests \"The Ravaged Caravan\" and \"Wildmane Totem\", and 1 other quest"
                .to_string()
        ),
        "{texts:?}"
    );
    assert!(
        texts.contains(&"Took on the quest \"Swoop Hunting\"".to_string()),
        "a quest finished the same day is told once, as completed: {texts:?}"
    );
}

#[test]
fn a_completed_quest_survives_a_crowded_day() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    let j = journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap();
    let awards = mrglglgl_core::milestones::compute(&j);
    let texts = beats::extract(&j, "2026-11-08", &awards, &Thresholds::default()).texts();
    assert!(
        texts.contains(&"Completed the quest \"The Ravaged Caravan\"".to_string()),
        "{texts:?}"
    );
    assert!(
        texts.contains(&"Looted Carving Knife".to_string()),
        "the rare find stays too: {texts:?}"
    );
}

#[test]
fn a_profession_the_client_lists_twice_is_told_once() {
    // The skills list gives some professions two IDs with one name and rank (Herbalism as 182 and
    // 2944, Skinning as 393 and 2947); the second is not a known profession ID.
    let mut day = Day::default();
    for (id, name, cat, from, to) in [
        (182, "Herbalism", "Professions", 30, 37),
        (2944, "Herbalism", "Professions", 30, 37),
        (393, "Skinning", "Professions", 80, 83),
        (2947, "Skinning", "Professions", 80, 83),
        (136, "Staves", "Weapon Skills", 31, 36),
    ] {
        day.skills.insert(
            id,
            SkillRecord {
                name: Some(name.into()),
                category: Some(cat.into()),
                from,
                to,
            },
        );
    }
    let j = journal(vec![("2026-11-17", day)]);
    let texts = beats::extract(&j, "2026-11-17", &[], &Thresholds::default()).texts();
    let herbalism: Vec<&String> = texts.iter().filter(|t| t.contains("Herbalism")).collect();
    assert_eq!(herbalism, ["Herbalism rose from 30 to 37"], "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t.starts_with("Practised") && t.contains("Skinning")),
        "{texts:?}"
    );
}
