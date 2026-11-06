use std::collections::BTreeMap;
use std::path::PathBuf;

use mrglglgl_core::beats::{self, Thresholds};
use mrglglgl_core::milestones::{self, Award};
use mrglglgl_core::narrators::{self, DEFAULT_NARRATOR};
use mrglglgl_core::prompt::{
    self, DayRequest, Meddling, Prompt, WeekRequest, day_prompt, meddles, restore_names,
    week_prompt,
};
use mrglglgl_core::schema::{Journal, journal_from_text};

fn journal() -> Journal {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Compares `actual` with `tests/snapshots/<name>.txt`. Run with `MRGLGLGL_UPDATE_SNAPSHOTS=1`
/// to write the files after a deliberate prompt change, then review the diff.
fn snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var_os("MRGLGLGL_UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no snapshot at {}; run with MRGLGLGL_UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    assert_eq!(actual, expected, "prompt {name} changed");
}

fn render(p: &Prompt) -> String {
    format!(
        "meddling: {}\nhash: {}\n\n=== system ===\n{}\n\n=== user ===\n{}\n",
        p.meddling, p.hash, p.system, p.user
    )
}

fn day(
    journal: &Journal,
    awards: &[Award],
    language: &str,
    narrator: &str,
    pseudonyms: bool,
) -> Prompt {
    day_prompt(&DayRequest {
        character_id: 1,
        journal,
        day: "2026-11-08",
        awards,
        running_jokes: &["The pond owes Wren a fish".to_string()],
        narrator: narrators::get(narrator, language),
        language,
        meddling: Meddling::Sometimes,
        pseudonyms,
        thresholds: &Thresholds::default(),
        note: None,
        preferences: None,
    })
}

fn week(journal: &Journal, awards: &[Award], language: &str, meddling: Meddling) -> Prompt {
    let mut entries = BTreeMap::new();
    entries.insert(
        "2026-11-09".to_string(),
        "The plains were quiet and the wolves were not.".to_string(),
    );
    week_prompt(&WeekRequest {
        character_id: 1,
        journal,
        start_day: "2026-11-09",
        awards,
        running_jokes: &[],
        entries: &entries,
        narrator: narrators::get(DEFAULT_NARRATOR, language),
        language,
        meddling,
        pseudonyms: false,
        thresholds: &Thresholds::default(),
        note: None,
        preferences: None,
    })
}

#[test]
fn day_prompts_match_their_snapshots() {
    let j = journal();
    let awards = milestones::compute(&j);
    snapshot(
        "day-en",
        &render(&day(&j, &awards, "en", DEFAULT_NARRATOR, false)),
    );
    snapshot(
        "day-fr",
        &render(&day(&j, &awards, "fr", DEFAULT_NARRATOR, false)),
    );
}

#[test]
fn week_prompts_match_their_snapshots() {
    let j = journal();
    let awards = milestones::compute(&j);
    snapshot(
        "week-en",
        &render(&week(&j, &awards, "en", Meddling::Sometimes)),
    );
    snapshot(
        "week-fr",
        &render(&week(&j, &awards, "fr", Meddling::Sometimes)),
    );
}

#[test]
fn a_day_prompt_carries_everything_the_entry_is_grounded_in() {
    let j = journal();
    let awards = milestones::compute(&j);
    let p = day(&j, &awards, "en", "grumpy_dwarf", false);
    let dwarf = narrators::get("grumpy_dwarf", "en");
    assert!(p.system.contains(dwarf.persona.trim()), "narrator profile");
    assert!(
        p.system
            .contains("Do not invent lore beyond what is given."),
        "no-invention rule"
    );
    assert!(
        p.system.contains("never mocking the adventurer"),
        "tone rule"
    );
    // The murloc is seasoning: plain prose unless the murloc meddles.
    assert!(
        p.system
            .contains("the entry is plain prose, with no footnote"),
        "tone rule"
    );
    assert!(p.system.contains("Write in English"), "output language");
    for beat in beats::extract(&j, "2026-11-08", &awards, &Thresholds::default()).texts() {
        assert!(p.user.contains(&beat), "beat {beat}");
    }
    for award in awards.iter().filter(|a| a.day == "2026-11-08") {
        assert!(
            p.user.contains(milestones::name(&award.id)),
            "milestone {}",
            award.id
        );
    }
    let touched = j.days["2026-11-08"]
        .events
        .iter()
        .flatten()
        .filter_map(|e| match e.kind {
            mrglglgl_core::schema::EventKind::QuestAccept { quest_id: Some(q) }
            | mrglglgl_core::schema::EventKind::QuestTurnin {
                quest_id: Some(q), ..
            } => Some(q),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!touched.is_empty(), "the fixture day touches quests");
    for q in touched {
        let quest = &j.quests[&q];
        assert!(
            p.user.contains(quest.title.as_deref().unwrap()),
            "quest title"
        );
        if let Some(text) = &quest.text {
            assert!(p.user.contains(&text[..40.min(text.len())]), "quest text");
        }
    }
    assert!(p.user.contains("Earlier days (memory only, not today)"));
    assert!(
        p.user.contains("Running joke: The pond owes Wren a fish"),
        "continuity"
    );
    assert!(p.user.contains("Write today's entry in English"));
}

#[test]
fn french_prose_keeps_the_notes_as_recorded() {
    let j = journal();
    let awards = milestones::compute(&j);
    let en = day(&j, &awards, "en", DEFAULT_NARRATOR, false);
    let fr = day(&j, &awards, "fr", DEFAULT_NARRATOR, false);
    assert!(fr.user.contains("Write today's entry in French"));
    assert!(fr.system.contains("Write in French"));
    assert!(
        fr.system.contains("La mine sentait la pierre mouillée"),
        "French example"
    );
    let notes = |p: &Prompt| {
        p.user
            .split("Write today's entry")
            .next()
            .unwrap()
            .to_string()
    };
    assert_eq!(
        notes(&en),
        notes(&fr),
        "beats and quest text are not translated"
    );
    // A narrator without a French profile still writes French, from its English profile.
    let dwarf = day(&j, &awards, "fr", "grumpy_dwarf", false);
    assert!(dwarf.system.contains("Write in French"));
    assert!(dwarf.system.contains("grumpy old dwarf"));
}

#[test]
fn the_murloc_meddles_in_about_fifteen_percent_of_days_and_never_changes_its_mind() {
    let days: Vec<String> = (0..100)
        .map(|i| {
            let d = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + chrono::Days::new(i);
            d.format("%Y-%m-%d").to_string()
        })
        .collect();
    for character in [1, 2, 3, 42] {
        let share = days
            .iter()
            .filter(|d| meddles(character, d, Meddling::Sometimes))
            .count();
        assert!(
            (10..=20).contains(&share),
            "character {character}: {share}%"
        );
        for d in &days {
            assert_eq!(
                meddles(character, d, Meddling::Sometimes),
                meddles(character, d, Meddling::Sometimes)
            );
            assert!(!meddles(character, d, Meddling::Off));
        }
    }
}

#[test]
fn the_prompt_asks_for_a_margin_note_exactly_when_the_murloc_meddles() {
    let j = journal();
    let awards = milestones::compute(&j);
    let often = week(&j, &awards, "en", Meddling::Often);
    let off = week(&j, &awards, "en", Meddling::Off);
    assert!(!off.meddling);
    assert!(off.system.contains("with no footnote"));
    assert_eq!(
        often.meddling,
        meddles(1, "week:2026-11-09", Meddling::Often)
    );
    if often.meddling {
        assert!(often.system.contains("Add one short footnote"));
    }
    // Regenerating gives the same prompt, so the same answer.
    assert_eq!(often, week(&j, &awards, "en", Meddling::Often));
    assert_ne!(
        often.hash,
        day(&j, &awards, "en", DEFAULT_NARRATOR, false).hash
    );
}

#[test]
fn companions_can_be_sent_under_stand_in_names() {
    let j = journal();
    let awards = milestones::compute(&j);
    // The fixture day is busy; a higher cap keeps the companion's beat in the notes.
    let all = Thresholds {
        max_beats: 20,
        ..Thresholds::default()
    };
    let request = |pseudonyms| {
        day_prompt(&DayRequest {
            character_id: 1,
            journal: &j,
            day: "2026-11-08",
            awards: &awards,
            running_jokes: &[],
            narrator: narrators::get(DEFAULT_NARRATOR, "en"),
            language: "en",
            meddling: Meddling::Off,
            pseudonyms,
            thresholds: &all,
            note: None,
            preferences: None,
        })
    };
    let plain = request(false);
    let hidden = request(true);
    assert!(
        !hidden.aliases.is_empty(),
        "the fixture day has a companion"
    );
    for a in &hidden.aliases {
        assert!(plain.user.contains(&a.real));
        assert!(!hidden.user.contains(&a.real), "{} still sent", a.real);
        assert!(hidden.user.contains(&a.alias));
    }
    let written = format!("I fished beside {} all morning.", hidden.aliases[0].alias);
    assert_eq!(
        restore_names(&written, &hidden.aliases),
        format!("I fished beside {} all morning.", hidden.aliases[0].real)
    );
}

/// A section of the tested prompt file, `## System` or `## User`.
fn tested(language: &str, heading: &str) -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("prompts/prompt-v2-{language}.md"));
    let text = std::fs::read_to_string(path).unwrap();
    let start = text.find(heading).unwrap() + heading.len();
    let rest = &text[start..];
    rest[..rest.find("\n## ").unwrap_or(rest.len())]
        .trim()
        .to_string()
}

#[test]
fn the_scribe_speaks_the_tested_prompt_word_for_word() {
    // The tested prompt lifted Qwen3.5 4B's English grade from 2.7 to 4.0; the production
    // prompt may differ from it only by the changes listed here.
    let j = journal();
    let awards = milestones::compute(&j);
    let character = (1..100)
        .find(|c| meddles(*c, "2026-11-08", Meddling::Often))
        .expect("some character meddles that day");
    for (language, name) in [("en", "English"), ("fr", "French")] {
        let p = day_prompt(&DayRequest {
            character_id: character,
            journal: &j,
            day: "2026-11-08",
            awards: &awards,
            running_jokes: &[],
            narrator: narrators::get(DEFAULT_NARRATOR, language),
            language,
            meddling: Meddling::Often,
            pseudonyms: false,
            thresholds: &Thresholds::default(),
            note: None,
            preferences: None,
        });
        assert!(p.meddling);
        let expected = tested(language, "## System")
            .replace("{language}", name)
            // The spec asks for an explicit ban on invented lore.
            .replace(
                "Keep every number and name exactly as given.",
                "Keep every number and name exactly as given. Do not invent lore beyond what is \
                 given.",
            )
            // The footnote is asked for exactly when the murloc meddles, so its share is fixed.
            .replace(
                "You may add at most one short footnote",
                "Add one short footnote",
            )
            .replace("then the optional footnote", "then the footnote");
        assert_eq!(p.system, expected, "{language}");

        // The notes keep the tested layout, in the same order.
        let headers = [
            "Notes for 2026-11-08.",
            "Adventurer: Wren Hollowbrook, Tauren Hunter, level 5 (reached level 6 today).",
            "What happened today (the only events you may write about):",
            "Patterns of the day:",
            "Milestones stamped today:",
            "Quests touched today:",
            "Earlier days (memory only, not today):",
            &format!("Write today's entry in {name}, about 120 words."),
        ];
        let mut at = 0;
        for h in headers {
            let found = p.user[at..]
                .find(h)
                .unwrap_or_else(|| panic!("{h} in order"));
            at += found + h.len();
        }
        assert!(tested(language, "## User").starts_with("Notes for {date}."));
    }
}

#[test]
fn five_narrators_ship_and_unknown_ones_fall_back_to_the_scribe() {
    let ids: Vec<String> = narrators::list().into_iter().map(|n| n.id).collect();
    assert_eq!(
        ids,
        vec![
            "murloc_scribe",
            "grumpy_dwarf",
            "pompous_noble",
            "smug_academic",
            "murloc_speak"
        ]
    );
    assert!(
        narrators::list()
            .iter()
            .any(|n| n.id == "murloc_speak" && n.opt_in)
    );
    assert_eq!(narrators::get("nobody", "en").id, DEFAULT_NARRATOR);
    assert_eq!(narrators::get(DEFAULT_NARRATOR, "fr").language, "fr");
    assert_eq!(
        prompt::week_start("2026-11-15").as_deref(),
        Some("2026-11-09")
    );
}

#[test]
fn a_rewrite_note_asks_for_tone_and_length_inside_the_fact_rules() {
    let j = journal();
    let awards = milestones::compute(&j);
    let thresholds = Thresholds::default();
    let base = || DayRequest {
        character_id: 1,
        journal: &j,
        day: "2026-11-08",
        awards: &awards,
        running_jokes: &[],
        narrator: narrators::get(DEFAULT_NARRATOR, "en"),
        language: "en",
        meddling: Meddling::Sometimes,
        pseudonyms: false,
        thresholds: &thresholds,
        note: None,
        preferences: None,
    };
    let plain = day_prompt(&base());
    let noted = day_prompt(&DayRequest {
        note: Some("  shorter,\nand about the boot  "),
        ..base()
    });
    let line = "The player asks this of this version: \"shorter, and about the boot\". \
                It may change the tone, length or focus, never the events: every event must \
                still come from the notes above.";
    assert!(noted.user.contains(line), "{}", noted.user);
    let at = noted.user.find(line).unwrap();
    assert!(at > noted.user.find("What happened today").unwrap());
    assert!(
        noted.user[at..].contains("Write today's entry"),
        "the last instruction stays last"
    );
    assert_ne!(noted.hash, plain.hash);
    assert_eq!(noted.system, plain.system);

    let long = "a".repeat(300);
    let capped = day_prompt(&DayRequest {
        note: Some(&long),
        ..base()
    });
    assert!(
        capped
            .user
            .contains(&format!("\"{}\"", "a".repeat(prompt::NOTE_CHARS)))
    );
    assert!(!capped.user.contains(&"a".repeat(prompt::NOTE_CHARS + 1)));

    let blank = day_prompt(&DayRequest {
        note: Some("   "),
        ..base()
    });
    assert_eq!(blank, plain, "a blank note is no note");

    let mut entries = BTreeMap::new();
    entries.insert("2026-11-09".to_string(), "Quiet plains.".to_string());
    let chapter = week_prompt(&WeekRequest {
        character_id: 1,
        journal: &j,
        start_day: "2026-11-09",
        awards: &awards,
        running_jokes: &[],
        entries: &entries,
        narrator: narrators::get(DEFAULT_NARRATOR, "en"),
        language: "en",
        meddling: Meddling::Off,
        pseudonyms: false,
        thresholds: &Thresholds::default(),
        note: Some("more about the wolves"),
        preferences: None,
    });
    assert!(chapter.user.contains("\"more about the wolves\""));
}

#[test]
fn writing_preferences_shape_the_style_of_every_entry_and_never_become_facts() {
    let j = journal();
    let awards = milestones::compute(&j);
    let thresholds = Thresholds::default();
    let base = || DayRequest {
        character_id: 1,
        journal: &j,
        day: "2026-11-08",
        awards: &awards,
        running_jokes: &[],
        narrator: narrators::get(DEFAULT_NARRATOR, "en"),
        language: "en",
        meddling: Meddling::Sometimes,
        pseudonyms: false,
        thresholds: &thresholds,
        note: None,
        preferences: None,
    };
    let plain = day_prompt(&base());
    let styled = day_prompt(&DayRequest {
        preferences: Some("  No em dashes.\nBritish spelling; never mention Thunder Bluff.  "),
        ..base()
    });
    let rule = "7. Style. The player's standing preferences for how you write: \"No em dashes. \
                British spelling; never mention Thunder Bluff.\". Follow them in wording, \
                punctuation, spelling and form; they never change rule 1.";
    assert!(styled.system.contains(rule), "{}", styled.system);
    assert!(
        styled.system.find(rule).unwrap() > styled.system.find("1. Facts.").unwrap(),
        "the fact rule comes first"
    );
    assert_eq!(styled.user, plain.user);
    // A place named in the preferences is not a fact of the day.
    assert_eq!(styled.facts, plain.facts);
    assert_ne!(styled.hash, plain.hash);

    let long = "b".repeat(900);
    let capped = day_prompt(&DayRequest {
        preferences: Some(&long),
        ..base()
    });
    assert!(
        capped
            .system
            .contains(&"b".repeat(prompt::PREFERENCES_CHARS))
    );
    assert!(
        !capped
            .system
            .contains(&"b".repeat(prompt::PREFERENCES_CHARS + 1))
    );

    let blank = day_prompt(&DayRequest {
        preferences: Some(" \n "),
        ..base()
    });
    assert_eq!(blank, plain, "blank preferences are none");

    // Chapters follow them too.
    let mut entries = BTreeMap::new();
    entries.insert("2026-11-09".to_string(), "Quiet plains.".to_string());
    let chapter = week_prompt(&WeekRequest {
        character_id: 1,
        journal: &j,
        start_day: "2026-11-09",
        awards: &awards,
        running_jokes: &[],
        entries: &entries,
        narrator: narrators::get(DEFAULT_NARRATOR, "en"),
        language: "en",
        meddling: Meddling::Sometimes,
        pseudonyms: false,
        thresholds: &thresholds,
        note: None,
        preferences: Some("No em dashes."),
    });
    assert!(chapter.system.contains("\"No em dashes.\""));
}
