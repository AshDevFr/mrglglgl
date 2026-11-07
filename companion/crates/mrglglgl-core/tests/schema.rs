use std::collections::BTreeSet;
use std::path::PathBuf;

use mrglglgl_core::schema::{
    self, EventKind, JournalError, Killer, OLDEST_SCHEMA, SUPPORTED_SCHEMA,
};
use mrglglgl_core::sv;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// The SavedVariables fixtures the addon generates, shared by the addon's specs and these tests.
fn shared(name: &str) -> String {
    read(repo().join("fixtures/savedvariables").join(name))
}

fn local(name: &str) -> String {
    read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data")
            .join(name),
    )
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn reads_a_real_session_captured_from_the_client() {
    let journal = schema::journal_from_text(&shared("real-session.lua")).unwrap();
    assert_eq!(journal.character.name.as_deref(), Some("Wren"));
    assert_eq!(journal.character.surname.as_deref(), Some("Hollowbrook"));
    let day = &journal.days["2026-11-05"];
    let events = day.events.as_ref().unwrap();
    assert_eq!(events[0].kind, EventKind::SessionStart);
    assert_eq!(events[0].time, 1_793_941_742);
    assert!(journal.discovered.contains("Bloodhoof Village"));
}

#[test]
fn reads_every_event_kind_kills_and_stats() {
    let journal = schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    assert_eq!(journal.schema, 1);
    assert_eq!(journal.character.class.as_deref(), Some("HUNTER"));
    assert_eq!(journal.stats.boots, 1);
    assert_eq!(journal.stats.deaths_by_murloc, 1);
    assert_eq!(
        journal.quests[&749].title.as_deref(),
        Some("The Ravaged Caravan")
    );
    assert_eq!(journal.companions["Tamsin-Reedwater"].days, 1);

    let kinds: BTreeSet<String> = journal
        .days
        .values()
        .flat_map(|d| d.events.iter().flatten())
        .map(|e| {
            serde_json::to_value(&e.kind).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    for kind in [
        "sessionStart",
        "sessionEnd",
        "level",
        "zone",
        "subzone",
        "discover",
        "death",
        "boss",
        "questAccept",
        "questTurnin",
        "fish",
        "loot",
        "groupJoin",
        "screenshot",
    ] {
        assert!(kinds.contains(kind), "no {kind} event read");
    }
    assert!(
        !kinds.contains("unknown"),
        "a documented kind was read as unknown"
    );

    let death_by_murloc = journal
        .days
        .values()
        .flat_map(|d| d.events.iter().flatten())
        .find_map(|e| match &e.kind {
            EventKind::Death {
                killer: Some(k), ..
            } => Some(k.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        death_by_murloc,
        Killer {
            name: Some("Murloc Streamrunner".into()),
            npc_id: Some(732),
            murloc: true
        }
    );

    let wolves = &journal.days["2026-11-08"].kills[&2958];
    assert_eq!(wolves.count, 23);
    assert_eq!(wolves.classification, "normal");
}

#[test]
fn reads_a_compacted_day_from_its_digest() {
    let journal = schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    let day = &journal.days["2026-11-01"];
    assert!(day.compacted, "the game compacted the day");
    let digest = day.digest.as_ref().unwrap();
    assert_eq!(digest.version, 2);
    assert_eq!(digest.counts.boots, 1);
    assert_eq!(digest.counts.deaths, 1);
    assert_eq!(digest.deaths[0].environment.as_deref(), Some("Drowning"));
    assert_eq!(digest.fish.len(), 2);
    assert_eq!(digest.fish[1].boot, Some(true));
    assert_eq!(digest.zones, vec!["Mulgore".to_string()]);
}

#[test]
fn reads_digest_kill_lists_and_flags() {
    let journal = schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    let digest = journal.days["2026-11-08"].digest.as_ref().unwrap();
    assert_eq!(digest.notable_kills.len(), 1);
    assert_eq!(
        digest.notable_kills[0].name.as_deref(),
        Some("Flatland Cougar")
    );
    assert_eq!(digest.notable_kills[0].classification, "elite");
    assert_eq!(digest.turned_in, vec![749]);
    assert_eq!(digest.loot[0].name.as_deref(), Some("Carving Knife"));
    assert!(!digest.only_fished);
}

#[test]
fn ignores_fields_and_keeps_events_a_newer_addon_added() {
    let journal = schema::journal_from_text(&shared("unknown-fields.lua")).unwrap();
    assert_eq!(journal.stats.boots, 1);

    let journal = schema::journal_from_text(&local("journal-unknown-field.lua")).unwrap();
    let events = journal.days["2026-11-05"].events.as_ref().unwrap();
    assert_eq!(
        events[0].kind,
        EventKind::Unknown {
            name: "fish_dance".into()
        }
    );
}

#[test]
fn reports_an_older_journal_as_needing_one_login() {
    let err = schema::journal_from_text(&shared("schema-0.lua")).unwrap_err();
    assert_eq!(err, JournalError::OlderSchema { found: 0 });
    assert!(err.to_string().contains("log in once with the addon"));
}

#[test]
fn refuses_a_journal_from_a_newer_addon_with_a_clear_message() {
    let err = schema::journal_from_text(&local("journal-newer.lua")).unwrap_err();
    assert_eq!(
        err,
        JournalError::NewerSchema {
            found: SUPPORTED_SCHEMA + 1,
            supported: SUPPORTED_SCHEMA
        }
    );
    assert!(err.to_string().contains("update the app"));
}

#[test]
fn a_bad_journal_fails_alone_and_others_still_load() {
    let files = [
        shared("synthetic-character.lua"),
        local("journal-newer.lua"),
        "MrglglglCharDB = { [\"days\"] = ".to_string(),
        shared("real-session.lua"),
    ];
    let results: Vec<_> = files.iter().map(|f| schema::journal_from_text(f)).collect();
    assert!(results[0].is_ok());
    assert!(matches!(results[1], Err(JournalError::NewerSchema { .. })));
    assert!(matches!(results[2], Err(JournalError::Parse(_))));
    assert!(results[3].is_ok());
}

#[test]
fn a_file_without_a_journal_is_not_a_journal() {
    let err = schema::journal_from_text(&shared("synthetic-account.lua")).unwrap_err();
    assert_eq!(err, JournalError::NoJournal);
}

#[test]
fn reads_the_account_settings() {
    let file = sv::parse(&shared("synthetic-account.lua")).unwrap();
    let settings = schema::account_settings_from(&file).unwrap();
    assert_eq!(settings.retention_days, Some(30));
    assert_eq!(settings.loot_quality, Some(3));
    assert_eq!(settings.record.get("kills"), Some(&true));
    assert_eq!(settings.screenshots.get("levelUp"), Some(&true));
}

#[test]
fn supports_exactly_the_documented_schema() {
    let doc = read(repo().join("docs/schema.md"));
    assert!(doc.contains(&format!("**Current journal schema: {SUPPORTED_SCHEMA}.**")));
    const { assert!(OLDEST_SCHEMA <= SUPPORTED_SCHEMA) };
}

#[test]
fn reads_profession_skill_ups_crafting_and_gathering() {
    let journal = schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    let day = &journal.days["2026-11-08"];
    let mining = &day.skills[&186];
    assert_eq!(
        (mining.name.as_deref(), mining.from, mining.to),
        (Some("Mining"), 40, 41)
    );
    assert_eq!(mining.category.as_deref(), Some("Professions"));
    assert_eq!(
        day.crafted[&1251].count, 5,
        "a craft reported twice counts once"
    );
    assert_eq!(day.gathered[&2770].via.as_deref(), Some("Mining"));
    let digest = day.digest.as_ref().unwrap();
    assert_eq!(
        (
            digest.counts.skill_ups,
            digest.counts.crafted,
            digest.counts.gathered
        ),
        (6, 5, 2)
    );
    assert_eq!(digest.gathered.len(), 2);
}

#[test]
fn reads_money_balances_and_auction_events() {
    let journal = schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    let day = &journal.days["2026-11-08"];
    assert_eq!(day.money.inc["auction"], 1840);
    assert_eq!(day.money.out["repair"], 128);
    assert_eq!((day.money.earned(), day.money.spent()), (2267, 223));
    let events = day.events.as_ref().unwrap();
    assert_eq!(events[0].balance, Some(2350));
    assert!(events.iter().any(|e| e.kind
        == EventKind::Auction {
            act: Some("sold".into()),
            name: Some("Copper Ore".into()),
        }));
    let digest = day.digest.as_ref().unwrap();
    assert_eq!(
        (
            digest.counts.money_in,
            digest.counts.money_out,
            digest.counts.auctions
        ),
        (2267, 223, 1)
    );
    assert_eq!((digest.bal_start, digest.bal_end), (Some(2350), Some(4394)));
    assert_eq!(digest.money, day.money);
    assert_eq!(digest.auctions[0].name.as_deref(), Some("Copper Ore"));
}

#[test]
fn reads_chat_lines_own_and_others() {
    let journal = schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    let day = &journal.days["2026-11-08"];
    let lines: Vec<_> = day
        .chat
        .iter()
        .map(|l| (l.channel.as_str(), l.who.as_deref(), l.text.as_str()))
        .collect();
    assert_eq!(
        lines,
        [
            ("say", None, "The wolves grow bold this season."),
            (
                "say",
                Some("Tamsin"),
                "Then we thin the pack before nightfall."
            ),
            ("emote", None, "checks the string of her bow."),
        ]
    );
    assert_eq!(day.digest.as_ref().unwrap().counts.chat, 3);
}

/// Every field and event kind `docs/schema.md` documents must be read by the model, or listed
/// here as deliberately skipped. A field added to the doc fails this test until it is handled.
#[test]
fn reads_every_documented_field() {
    let handled: BTreeSet<&str> = [
        // MrglglglDB
        "schema",
        "settings.record.sessions",
        "settings.record.levels",
        "settings.record.quests",
        "settings.record.questText",
        "settings.record.zones",
        "settings.record.kills",
        "settings.record.deaths",
        "settings.record.fishing",
        "settings.record.loot",
        "settings.record.group",
        "settings.record.professions",
        "settings.record.money",
        "settings.record.chat",
        "settings.chat.say",
        "settings.chat.emote",
        "settings.chat.yell",
        "settings.chat.party",
        "settings.chat.raid",
        "settings.chat.guild",
        "settings.chat.officer",
        "settings.chat.whisper",
        "settings.chat.channel",
        "settings.chat.sayOthers",
        "settings.chat.emoteOthers",
        "settings.chat.yellOthers",
        "settings.chat.partyOthers",
        "settings.chat.raidOthers",
        "settings.chat.guildOthers",
        "settings.chat.officerOthers",
        "settings.chat.whisperOthers",
        "settings.chat.channelOthers",
        "settings.lootQuality",
        "settings.retentionDays",
        "settings.screenshots.levelUp",
        "settings.screenshots.death",
        "settings.screenshots.bossKill",
        "settings.screenshots.firstBoot",
        "settings.screenshots.firstMurlocKill",
        "settings.screenshots.firstDeathByMurloc",
        // MrglglglCharDB
        "char.name",
        "char.surname",
        "char.realm",
        "char.realmId",
        "char.race",
        "char.class",
        "char.faction",
        "char.locale",
        "days",
        "quests",
        "companions",
        "discovered",
        "stats.boots",
        "stats.murlocsSlain",
        "stats.deathsByMurloc",
        "stats.drownings",
        "addon.version",
        "addon.build",
        "addon.interface",
        "addon.login",
        "errors.n",
        "errors.last",
        // Day, event, kill aggregate, digest, quest, companion
        "ev",
        "kills",
        "anonKills",
        "c",
        "skills",
        "crafted",
        "gathered",
        "digest",
        "t",
        "k",
        "z",
        "sz",
        "m",
        "lv",
        "n",
        "name",
        "cls",
        "murloc",
        "v",
        "counts",
        "levelFrom",
        "levelTo",
        "zones",
        "subzones",
        "accepted",
        "turnedIn",
        "deaths",
        "notableKills",
        "fish",
        "loot",
        "bosses",
        "onlyFished",
        "nothingHappened",
        "thin",
        "title",
        "text",
        "first",
        "last",
        // Professions
        "skills[id].name",
        "skills[id].cat",
        "skills[id].from",
        "skills[id].to",
        "crafted[id].n",
        "gathered[id].n",
        "crafted[id].name",
        "gathered[id].name",
        "gathered[id].via",
        // Event kinds
        "session_start",
        "session_end",
        "level",
        "zone",
        "subzone",
        "discover",
        "death",
        "boss",
        "quest_accept",
        "quest_turnin",
        "group_join",
        "screenshot",
        "auction",
        // Event fields beyond the common ones
        "area",
        "src",
        "killer",
        "env",
        "enc",
        "q",
        "xp",
        "money",
        "item",
        "junk",
        "boot",
        "ql",
        "qty",
        "who",
        "moment",
        // Quest record fields beyond title, text and last
        "kind",
        // Money
        "bal",
        "act",
        "inc",
        "out",
        "moneyIn",
        "moneyOut",
        "auctions",
        "balStart",
        "balEnd",
        // Chat
        "chat",
        "ch",
        "msg",
        "to",
        "cn",
    ]
    .into_iter()
    .collect();

    let doc = read(repo().join("docs/schema.md"));
    let mut documented = BTreeSet::new();
    for line in doc.lines() {
        // The first cell of a table row holds the field (or event kind) names, in backticks; for
        // event kinds the second cell names the extra fields. Type cells carry no backticks.
        let Some(row) = line.strip_prefix("| ") else {
            continue;
        };
        for cell in row.split(" | ").take(2) {
            for name in cell.split('`').skip(1).step_by(2) {
                documented.insert(name.to_string());
            }
        }
    }
    // Status-value rows and the header row carry no backticked names, so this only collects
    // field and kind names.
    let missing: Vec<_> = documented
        .iter()
        .filter(|f| !handled.contains(f.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/schema.md documents fields the model does not read: {missing:?}"
    );
}

#[test]
fn reads_what_the_addon_notes_about_itself() {
    let j = schema::journal_from_text(
        r#"MrglglglCharDB = { schema = 1, char = {}, days = {},
            addon = { version = "addon-v0.2.0", build = "70790", interface = 16001, login = 1794344400 },
            errors = { n = 3, last = { msg = "Kills.lua:88: boom", event = "LOOT_OPENED", t = 1794344500 } } }"#,
    )
    .unwrap();
    let addon = j.addon.unwrap();
    assert_eq!(addon.version.as_deref(), Some("addon-v0.2.0"));
    assert_eq!(
        (addon.interface, addon.login),
        (Some(16001), Some(1_794_344_400))
    );
    let errors = j.errors.unwrap();
    assert_eq!(errors.count, 3);
    assert_eq!(errors.last_event.as_deref(), Some("LOOT_OPENED"));
    assert!(errors.last_message.unwrap().contains("boom"));
}

#[test]
fn reads_a_quests_level_and_kind() {
    let journal =
        mrglglgl_core::schema::journal_from_text(&shared("synthetic-character.lua")).unwrap();
    let caravan = &journal.quests[&749];
    assert_eq!(caravan.title.as_deref(), Some("The Ravaged Caravan"));
    assert_eq!(caravan.level, Some(6));
    assert_eq!(caravan.kind, None, "an ordinary quest has no kind");
    let text = "MrglglglCharDB = { [\"schema\"] = 1, [\"quests\"] = { [800] = { [\"title\"] = \"Into the Mines\", [\"lv\"] = 18, [\"kind\"] = \"dungeon\" } } }";
    let dungeon = &mrglglgl_core::schema::journal_from_text(text)
        .unwrap()
        .quests[&800];
    assert_eq!(
        (dungeon.level, dungeon.kind.as_deref()),
        (Some(18), Some("dungeon"))
    );
}
