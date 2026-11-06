use std::path::PathBuf;

use mrglglgl_core::export::{self, ExportError, ExportInput, FOOTER};
use mrglglgl_core::milestones;
use mrglglgl_core::schema::{Journal, journal_from_text};
use mrglglgl_core::stories::{Story, StoryKind, StoryStatus};
use mrglglgl_core::writer::CheckStatus;

fn journal() -> Journal {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    journal_from_text(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn story(id: i64, kind: StoryKind, day: &str, text: &str, check: CheckStatus) -> Story {
    Story {
        id,
        character_id: 1,
        kind,
        start_day: day.into(),
        end_day: day.into(),
        narrator: "murloc_scribe".into(),
        language: "en".into(),
        provider: "fake".into(),
        model: "fake-model".into(),
        prompt_hash: "h".into(),
        text: text.into(),
        created_at: 0,
        check,
        check_reason: None,
        status: StoryStatus::Accepted,
        note: None,
    }
}

const ENTRY: &str = "The wolves of Mulgore kept me busy.\n\nA murloc had the last word.";

#[test]
fn copy_entry_has_the_day_the_entry_and_a_removable_footer() {
    let text = export::copy_entry("2026-11-08", "en", ENTRY, true);
    assert_eq!(
        text,
        format!(
            "Sunday, November 8, 2026\n\n{ENTRY}\n\nvia Mrglglgl (github.com/AshDevFr/mrglglgl)"
        )
    );
    assert_eq!(text.lines().last(), Some(FOOTER), "one line");
    let bare = export::copy_entry("2026-11-08", "en", ENTRY, false);
    assert!(!bare.contains("Mrglglgl"));
    assert!(bare.ends_with("A murloc had the last word."));
    assert!(
        export::copy_entry("2026-11-08", "fr", "Mrgl.", false)
            .starts_with("dimanche 8 novembre 2026")
    );
}

fn input<'a>(
    j: &'a Journal,
    stories: &'a [Story],
    awards: &'a [milestones::Award],
) -> ExportInput<'a> {
    ExportInput {
        title: "Wren Hollowbrook",
        journal: j,
        stories,
        awards,
        language: "en",
    }
}

#[test]
fn markdown_has_every_day_with_its_newest_entry_or_its_notes() {
    let j = journal();
    let awards = milestones::compute(&j);
    let stories = vec![
        story(
            1,
            StoryKind::Day,
            "2026-11-08",
            "An older version.",
            CheckStatus::Passed,
        ),
        story(
            2,
            StoryKind::Day,
            "2026-11-08",
            ENTRY,
            CheckStatus::Rewritten,
        ),
        story(
            3,
            StoryKind::Day,
            "2026-11-09",
            "I sailed to Booty Bay.",
            CheckStatus::Failed,
        ),
        story(
            4,
            StoryKind::Week,
            "2026-11-02",
            "A week of wolves.",
            CheckStatus::Passed,
        ),
    ];
    let md = export::markdown(&input(&j, &stories, &awards));
    assert!(md.starts_with("# Wren Hollowbrook\n"));
    assert!(md.contains("## Week of Monday, November 2, 2026\n\nA week of wolves.\n"));
    assert!(md.contains(&format!("### Sunday, November 8, 2026\n\n{ENTRY}\n")));
    assert!(!md.contains("An older version."));
    // The set-aside entry is not exported; the day shows its notes.
    assert!(!md.contains("Booty Bay"));
    assert!(md.contains("### Monday, November 9, 2026\n\n*Notes from the day*\n\n- "));
    assert!(md.contains("Stamps: "));
    for day in j.days.keys() {
        assert!(
            md.contains(&export::long_date(day, "en")),
            "{day} is in the export"
        );
    }
}

#[test]
fn drafts_and_discarded_drafts_are_never_exported() {
    let j = journal();
    let awards = milestones::compute(&j);
    let mut draft = story(
        2,
        StoryKind::Day,
        "2026-11-08",
        "A draft.",
        CheckStatus::Passed,
    );
    draft.status = StoryStatus::Draft;
    let mut thrown = story(
        3,
        StoryKind::Day,
        "2026-11-08",
        "Thrown away.",
        CheckStatus::Passed,
    );
    thrown.status = StoryStatus::Discarded;
    let stories = vec![
        story(1, StoryKind::Day, "2026-11-08", ENTRY, CheckStatus::Passed),
        draft,
        thrown,
    ];
    let md = export::markdown(&input(&j, &stories, &awards));
    assert!(md.contains(ENTRY), "the accepted entry is exported");
    assert!(!md.contains("A draft."));
    assert!(!md.contains("Thrown away."));
}

#[test]
fn html_is_one_self_contained_page_with_an_open_licence_font() {
    let j = journal();
    let awards = milestones::compute(&j);
    let stories = vec![story(
        1,
        StoryKind::Day,
        "2026-11-08",
        "Wolves & <murlocs>.",
        CheckStatus::Passed,
    )];
    let page = export::html(&input(&j, &stories, &awards));
    assert!(page.starts_with("<!doctype html>"));
    assert!(page.contains("<title>Wren Hollowbrook</title>"));
    assert!(
        page.contains("src: url(data:font/woff2;base64,d09GMg"),
        "Literata is embedded"
    );
    assert!(
        page.contains("<p>Wolves &amp; &lt;murlocs&gt;.</p>"),
        "text is escaped"
    );
    // Nothing is fetched from elsewhere.
    assert!(!page.contains("http://") && !page.contains("https://"));
    assert!(!page.contains("<script"));
    assert!(!page.contains("<link"));
}

#[test]
fn exports_are_written_outside_the_game_folder_only() {
    let j = journal();
    let awards = milestones::compute(&j);
    let stories = vec![story(
        1,
        StoryKind::Day,
        "2026-11-08",
        ENTRY,
        CheckStatus::Passed,
    )];
    let game = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let roots = vec![game.path().to_path_buf()];

    let files = export::write(elsewhere.path(), &roots, &input(&j, &stories, &awards)).unwrap();
    assert_eq!(
        files.markdown,
        elsewhere.path().join("Wren-Hollowbrook-journal.md")
    );
    assert_eq!(
        files.html,
        elsewhere.path().join("Wren-Hollowbrook-journal.html")
    );
    assert!(
        std::fs::read_to_string(&files.markdown)
            .unwrap()
            .contains(ENTRY.lines().next().unwrap())
    );
    assert!(
        std::fs::read_to_string(&files.html)
            .unwrap()
            .contains("<html")
    );

    let inside = game.path().join("Screenshots");
    std::fs::create_dir_all(&inside).unwrap();
    let refused = export::write(&inside, &roots, &input(&j, &stories, &awards));
    assert!(matches!(refused, Err(ExportError::InsideGameFolder(_))));
    assert_eq!(
        std::fs::read_dir(&inside).unwrap().count(),
        0,
        "nothing written there"
    );
}
