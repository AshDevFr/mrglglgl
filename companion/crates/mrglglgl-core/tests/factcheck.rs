use std::path::PathBuf;
use std::time::Duration;

use mrglglgl_core::beats::Thresholds;
use mrglglgl_core::factcheck::{FactSet, check};
use mrglglgl_core::milestones;
use mrglglgl_core::narrators::{self, DEFAULT_NARRATOR};
use mrglglgl_core::prompt::{DayRequest, Meddling, Prompt, day_prompt};
use mrglglgl_core::schema::{Journal, journal_from_text};
use mrglglgl_core::writer::{CheckStatus, StreamEvent, WriteOptions, WriteResult, write};
use mrglglgl_llm::CancellationToken;
use mrglglgl_llm::fake::FakeProvider;
use tokio::sync::mpsc;

fn journal() -> Journal {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The fixture's 2026-11-08: 23 Prairie Wolves, a murloc death in Red Rocks, Mulgore.
fn prompt(language: &str) -> Prompt {
    let j = journal();
    let awards = milestones::compute(&j);
    day_prompt(&DayRequest {
        character_id: 1,
        journal: &j,
        day: "2026-11-08",
        awards: &awards,
        running_jokes: &[],
        narrator: narrators::get(DEFAULT_NARRATOR, language),
        language,
        meddling: Meddling::Off,
        pseudonyms: false,
        thresholds: &Thresholds::default(),
    })
}

fn options() -> WriteOptions {
    WriteOptions {
        language: "en".into(),
        max_tokens: 300,
        temperature: 0.4,
        timeout: Duration::from_secs(10),
        allowed: vec!["Horde".into()],
    }
}

const TRUE_ENTRY: &str = "The plains of Mulgore were busy today. I put down 23 Prairie Wolf \
    after Prairie Wolf, and a Flatland Cougar too, before a Murloc Streamrunner caught me \
    napping in Red Rocks. Still, I reached level 6, and Hogger will not be bothering anyone.";

#[test]
fn an_entry_that_sticks_to_the_notes_passes() {
    let p = prompt("en");
    let facts = FactSet::from_notes(&p.user, &[]);
    assert_eq!(check(TRUE_ENTRY, &facts), Vec::<String>::new());
    // Feelings, the light and ordinary sentence openers are not claims.
    let cosy = "Morning light lay over the grass. Then Prairie Wolves came, twenty-three of \
        them. I felt tired but proud, and one small fish would have been nice.";
    // Written by Qwen3.5 4B on this very day: the narrator's "I've" is not a name.
    let real = "The air in Red Rocks tasted of dust, a place I've never seen before. I'm sure I'll \
        be back, and I'd bring a lantern.";
    assert_eq!(check(real, &facts), Vec::<String>::new());
    // From the narrator samples: a sentence opener followed by "I" is not a name.
    assert_eq!(
        check(
            "Though I fell, I rose. Finally I rested in Red Rocks. Still, I'd do it again.",
            &facts
        ),
        Vec::<String>::new()
    );
    let whole = "The air in Red Rocks tasted of dust, a place I've never seen before where Prairie \
        Wolves waited like grumpy neighbors. I spent the morning clearing the path, killing \
        twenty-three of them and a Murloc Streamrunner, until the elite Flatland Cougar decided to \
        test my patience. Then came Hogger, a tough fight that ended with a Carving Knife in my \
        pack, a quiet treasure for a Tauren like me.\n\nBut the day turned sour when a Murloc \
        Streamrunner finally caught me in Red Rocks, sending me back to the start. Reaching level \
        6 felt like a warm sun after a cold rain, and seeing the milestones \"First Blood (Fishy)\" \
        and \"Mrglglgl'd\" stamped on my record made the stumble worth it.";
    assert_eq!(check(whole, &facts), Vec::<String>::new());
    assert_eq!(check(cosy, &facts), Vec::<String>::new());
}

#[test]
fn invented_numbers_and_names_are_caught() {
    let p = prompt("en");
    let facts = FactSet::from_notes(&p.user, &[]);
    let invented = "I killed 17 wolves near Thunder Bluff, then sailed to Booty Bay with \
        seventeen more pelts. Copper Ore glittered everywhere.";
    assert_eq!(
        check(invented, &facts),
        vec!["17", "Booty Bay", "seventeen", "Copper Ore"]
    );
}

#[test]
fn french_number_words_count_too() {
    let p = prompt("fr");
    let facts = FactSet::from_notes(&p.user, &[]);
    assert!(check("J'ai abattu vingt-trois Prairie Wolf.", &facts).is_empty());
    assert_eq!(
        check("J'ai abattu dix-sept Prairie Wolf.", &facts),
        vec!["dix-sept"]
    );
}

#[tokio::test]
async fn a_wrong_count_is_rewritten_once_and_passes() {
    let p = prompt("en");
    let wrong = TRUE_ENTRY.replace("23", "17");
    let provider = FakeProvider::new([wrong.as_str(), TRUE_ENTRY]);
    let (tx, mut rx) = mpsc::channel(64);
    let result = write(
        &provider,
        &p,
        &options(),
        &CancellationToken::new(),
        Some(&tx),
    )
    .await
    .unwrap();
    assert_eq!(
        result,
        WriteResult::Written {
            text: TRUE_ENTRY.to_string(),
            check: CheckStatus::Rewritten,
            reason: None,
        }
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 2, "one rewrite, no more");
    assert!(
        requests[1].user.contains("not in the notes: 17"),
        "the claim is named"
    );
    assert!(
        requests[1].user.contains(&wrong),
        "the first draft is shown"
    );
    drop(tx);
    let mut events = Vec::new();
    while let Some(e) = rx.recv().await {
        events.push(e);
    }
    assert!(
        events.contains(&StreamEvent::Restart),
        "the watcher sees the restart"
    );
}

#[tokio::test]
async fn a_zone_never_visited_fails_twice_and_the_day_keeps_its_facts() {
    let p = prompt("en");
    let elsewhere = "I spent the whole day wandering Elwynn Forest, far from Mulgore.";
    let provider = FakeProvider::new([elsewhere, elsewhere]);
    let result = write(&provider, &p, &options(), &CancellationToken::new(), None)
        .await
        .unwrap();
    let WriteResult::Written { check, reason, .. } = result else {
        panic!("written, not stopped: {result:?}");
    };
    assert_eq!(check, CheckStatus::Failed);
    assert_eq!(
        reason.as_deref(),
        Some("mentions Elwynn Forest not in the day's notes")
    );
    assert_eq!(provider.requests().len(), 2);
}

#[tokio::test]
async fn reasoning_blocks_and_padding_are_stripped() {
    let p = prompt("en");
    let provider = FakeProvider::new([format!("<think>\n\n</think>\n\n  {TRUE_ENTRY}\n")]);
    let result = write(&provider, &p, &options(), &CancellationToken::new(), None)
        .await
        .unwrap();
    assert!(matches!(
        result,
        WriteResult::Written { ref text, check: CheckStatus::Passed, .. } if text == TRUE_ENTRY
    ));
}

#[tokio::test]
async fn a_cancelled_write_stores_nothing_to_check() {
    let p = prompt("en");
    let provider = FakeProvider::new([TRUE_ENTRY]);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = write(&provider, &p, &options(), &cancel, None)
        .await
        .unwrap();
    assert_eq!(result, WriteResult::Cancelled);
}
