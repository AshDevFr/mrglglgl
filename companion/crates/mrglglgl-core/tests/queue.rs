use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mrglglgl_core::archive;
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::game::is_game_process;
use mrglglgl_core::queue::{
    self, Ask, CANCELLED, Control, JobState, MAX_ATTEMPTS, PreviewDay, QueueEvent, drain, run_next,
};
use mrglglgl_core::schema::{Character, Day, Event, EventKind, Journal};
use mrglglgl_core::stories::{self, NewStory, StoryKind, StoryStatus};
use mrglglgl_core::writer::{CheckStatus, WriteResult};
use mrglglgl_llm::ProviderError;
use mrglglgl_llm::fake::{FakeProvider, Pace};

const DAYS: [&str; 5] = [
    "2026-11-02",
    "2026-11-03",
    "2026-11-04",
    "2026-11-05",
    "2026-11-06",
];
const TODAY: &str = "2026-11-07";
const ENTRY: &str = "A quiet afternoon by the water, and the fish were kind.";

fn fishing_day() -> Day {
    Day {
        events: Some(vec![Event {
            time: 1_794_160_000,
            zone: Some("Mulgore".into()),
            subzone: None,
            map_id: None,
            level: Some(9),
            balance: None,
            kind: EventKind::Fish {
                item_id: Some(6291),
                name: Some("Raw Brilliant Smallfish".into()),
                junk: Some(false),
                boot: Some(false),
            },
        }]),
        ..Day::default()
    }
}

/// Five finished days and today.
fn journal() -> Journal {
    let mut j = Journal {
        schema: 1,
        character: Character {
            name: Some("Wren".into()),
            surname: Some("Hollowbrook".into()),
            ..Character::default()
        },
        ..Journal::default()
    };
    for day in DAYS.iter().chain([&TODAY]) {
        j.days.insert(day.to_string(), fishing_day());
    }
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

type Events = Arc<Mutex<Vec<QueueEvent>>>;

fn recorder() -> (Events, impl Fn(QueueEvent) + Send + Sync) {
    let events: Events = Arc::default();
    let log = events.clone();
    (events, move |e| log.lock().unwrap().push(e))
}

fn notes_day(request: &mrglglgl_llm::GenerateRequest) -> String {
    request
        .user
        .lines()
        .next()
        .unwrap()
        .rsplit(' ')
        .next()
        .unwrap()
        .trim_end_matches('.')
        .to_string()
}

#[test]
fn finished_days_are_queued_but_never_today_or_an_unfinished_week() {
    let (db, id) = setup();
    let db = db.lock().unwrap();
    assert_eq!(
        queue::enqueue_finished(&db, id, &journal(), TODAY).unwrap(),
        5
    );
    let jobs = queue::jobs(&db, id).unwrap();
    let days: Vec<&str> = jobs.iter().map(|j| j.start_day.as_str()).collect();
    assert_eq!(days, DAYS);
    assert!(jobs.iter().all(|j| j.kind == StoryKind::Day));
    assert_eq!(jobs[0].position, Some(1));
    // Queuing again adds nothing.
    assert_eq!(
        queue::enqueue_finished(&db, id, &journal(), TODAY).unwrap(),
        0
    );
    // Once the week is over, its chapter is queued after its days.
    assert_eq!(
        queue::enqueue_finished(&db, id, &journal(), "2026-11-09").unwrap(),
        2
    );
    let jobs = queue::jobs(&db, id).unwrap();
    assert_eq!(jobs.last().unwrap().kind, StoryKind::Week);
    assert_eq!(jobs.last().unwrap().start_day, "2026-11-02");
}

#[tokio::test]
async fn five_days_are_written_one_at_a_time_oldest_first() {
    let (db, id) = setup();
    queue::enqueue_finished(&db.lock().unwrap(), id, &journal(), TODAY).unwrap();
    let provider = FakeProvider::new([ENTRY]);
    let control = Control::new(false);
    let (events, on_event) = recorder();
    let ran = drain(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    assert_eq!(ran, 5);

    let order: Vec<String> = provider.requests().iter().map(notes_day).collect();
    assert_eq!(order, DAYS);
    // One at a time: each job finishes before the next starts.
    let mut running = 0;
    for e in events.lock().unwrap().iter() {
        match e {
            QueueEvent::Started { .. } => {
                running += 1;
                assert_eq!(running, 1, "two jobs ran at once");
            }
            QueueEvent::Finished { outcome, .. } => {
                assert_eq!(outcome, "written");
                running -= 1;
            }
            _ => {}
        }
    }
    let db = db.lock().unwrap();
    assert!(queue::jobs(&db, id).unwrap().is_empty());
    let written = stories::latest(&db, id, StoryKind::Day).unwrap();
    assert_eq!(written.keys().collect::<Vec<_>>(), DAYS);
    let story = &written[DAYS[0]];
    assert_eq!(story.text, ENTRY);
    assert_eq!(story.provider, "fake");
    assert_eq!(story.narrator, "murloc_scribe");
    assert_eq!(story.check, CheckStatus::Passed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pausing_stops_after_the_current_entry() {
    let (db, id) = setup();
    queue::enqueue_finished(&db.lock().unwrap(), id, &journal(), TODAY).unwrap();
    let provider = FakeProvider::new([ENTRY]).with_pace(Pace::PerWord(Duration::from_millis(20)));
    let control = Arc::new(Control::new(false));
    let pause = control.clone();
    let on_event = move |e: QueueEvent| {
        if matches!(e, QueueEvent::Started { .. }) {
            pause.set_paused(true);
        }
    };
    let ran = drain(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    assert_eq!(ran, 1, "the entry in progress finished, no other started");
    let db = db.lock().unwrap();
    assert_eq!(stories::latest(&db, id, StoryKind::Day).unwrap().len(), 1);
    let left = queue::jobs(&db, id).unwrap();
    assert_eq!(left.len(), 4);
    assert!(left.iter().all(|j| j.state == JobState::Pending));
}

#[tokio::test]
async fn failures_are_retried_with_a_pause_then_marked_failed_in_plain_words() {
    let (db, id) = setup();
    queue::request(
        &db.lock().unwrap(),
        id,
        StoryKind::Day,
        DAYS[0],
        &Ask::default(),
    )
    .unwrap();
    let provider = FakeProvider::scripted([Err(ProviderError::Unreachable("gpu-box".into()))]);
    let control = Control::new(false);
    let (_, on_event) = recorder();

    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap()
        .unwrap();
    {
        let db = db.lock().unwrap();
        let job = &queue::jobs(&db, id).unwrap()[0];
        assert_eq!(job.state, JobState::Pending);
        assert_eq!(job.attempts, 1);
        assert_eq!(job.last_error.as_deref(), Some("could not reach gpu-box"));
        // It waits before trying again.
        assert!(
            queue::take_next(&db, chrono::Utc::now().timestamp())
                .unwrap()
                .is_none()
        );
        assert!(
            queue::next_due_in(&db, chrono::Utc::now().timestamp())
                .unwrap()
                .unwrap()
                > 0
        );
    }
    // The remaining attempts, as if the pauses had passed.
    for _ in 1..MAX_ATTEMPTS {
        let db = db.lock().unwrap();
        let job = queue::take_next(&db, i64::MAX)
            .unwrap()
            .expect("due eventually");
        queue::fail(&db, &job, "could not reach gpu-box", 0).unwrap();
    }
    let db = db.lock().unwrap();
    let job = &queue::jobs(&db, id).unwrap()[0];
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.attempts, MAX_ATTEMPTS);
    assert_eq!(job.position, None);
    assert!(
        queue::take_next(&db, i64::MAX).unwrap().is_none(),
        "failed jobs wait for the player"
    );

    queue::retry(&db, id, StoryKind::Day, DAYS[0]).unwrap();
    let job = &queue::jobs(&db, id).unwrap()[0];
    assert_eq!((job.state, job.attempts), (JobState::Pending, 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_entry_stores_nothing_and_waits_for_the_player() {
    let (db, id) = setup();
    queue::request(
        &db.lock().unwrap(),
        id,
        StoryKind::Day,
        DAYS[0],
        &Ask::default(),
    )
    .unwrap();
    let long = "word ".repeat(200);
    let provider =
        FakeProvider::new([long.as_str()]).with_pace(Pace::PerWord(Duration::from_millis(10)));
    let control = Arc::new(Control::new(false));
    let canceller = control.clone();
    let on_event = move |e: QueueEvent| {
        if let QueueEvent::Chunk {
            character_id,
            kind,
            start_day,
            ..
        } = e
        {
            canceller.cancel(Some((character_id, kind, &start_day)));
        }
    };
    let started = std::time::Instant::now();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    let db = db.lock().unwrap();
    assert!(stories::all(&db, id).unwrap().is_empty(), "nothing stored");
    let job = &queue::jobs(&db, id).unwrap()[0];
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.last_error.as_deref(), Some(CANCELLED));
    assert_eq!(job.attempts, 0, "a cancel is not a failed attempt");
}

#[tokio::test]
async fn regenerating_keeps_the_old_version_and_jumps_the_line() {
    let (db, id) = setup();
    queue::enqueue_finished(&db.lock().unwrap(), id, &journal(), TODAY).unwrap();
    let provider = FakeProvider::new([ENTRY, "The water was patient today, and so was I."]);
    let control = Control::new(false);
    let (_, on_event) = recorder();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();

    // Ask for the first day again, with another narrator, while four days wait.
    queue::request(
        &db.lock().unwrap(),
        id,
        StoryKind::Day,
        DAYS[0],
        &Ask {
            narrator: Some("grumpy_dwarf"),
            ..Ask::default()
        },
    )
    .unwrap();
    let job = run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.start_day, DAYS[0], "the request ran before the backlog");
    let db = db.lock().unwrap();
    let versions = stories::versions(&db, id, StoryKind::Day, DAYS[0]).unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(
        versions[0].text,
        "The water was patient today, and so was I."
    );
    assert_eq!(versions[0].narrator, "grumpy_dwarf");
    assert_eq!(versions[1].text, ENTRY);
}

#[tokio::test]
async fn an_entry_that_fails_the_fact_check_is_kept_with_its_reason() {
    let (db, id) = setup();
    queue::request(
        &db.lock().unwrap(),
        id,
        StoryKind::Day,
        DAYS[0],
        &Ask::default(),
    )
    .unwrap();
    let invented = "I sailed all the way to Booty Bay today.";
    let provider = FakeProvider::new([invented]);
    let control = Control::new(false);
    let (events, on_event) = recorder();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    let db = db.lock().unwrap();
    let story = &stories::versions(&db, id, StoryKind::Day, DAYS[0]).unwrap()[0];
    assert_eq!(story.check, CheckStatus::Failed);
    assert_eq!(
        story.check_reason.as_deref(),
        Some("mentions Booty Bay not in the day's notes")
    );
    assert!(events.lock().unwrap().iter().any(|e| matches!(
        e,
        QueueEvent::Finished { outcome, .. } if outcome == "setAside"
    )));
}

#[test]
fn interrupted_jobs_go_back_in_line_after_a_restart() {
    let (db, id) = setup();
    let db = db.lock().unwrap();
    queue::enqueue_finished(&db, id, &journal(), TODAY).unwrap();
    let job = queue::take_next(&db, 0).unwrap().unwrap();
    assert_eq!(queue::jobs(&db, id).unwrap()[0].state, JobState::Running);
    queue::recover(&db).unwrap();
    let again = queue::take_next(&db, 0).unwrap().unwrap();
    assert_eq!(again.start_day, job.start_day);
}

#[test]
fn the_game_is_recognised_by_its_process_name() {
    for name in [
        "Wow.exe",
        "WowClassic.exe",
        "WowClassicT.exe",
        "WowT.exe",
        "Wow-64.exe",
        "World of Warcraft",
        "World of Warcraft Classic",
    ] {
        assert!(is_game_process(name), "{name}");
    }
    for name in [
        "Battle.net.exe",
        "Mrglglgl",
        "firefox",
        "Blizzard Agent",
        "wow2.exe",
    ] {
        assert!(!is_game_process(name), "{name}");
    }
}

fn review_mode(db: &Database) {
    let mut settings = db.settings().unwrap();
    settings.writing.review = true;
    db.save_settings(&settings).unwrap();
}

#[tokio::test]
async fn in_automatic_mode_entries_go_straight_into_the_journal() {
    let (db, id) = setup();
    queue::enqueue_finished(&db.lock().unwrap(), id, &journal(), TODAY).unwrap();
    let provider = FakeProvider::new([ENTRY]);
    let (_, on_event) = recorder();
    run_next(&db, &provider, &Control::new(false), "en", &on_event)
        .await
        .unwrap();
    let db = db.lock().unwrap();
    let story = &stories::versions(&db, id, StoryKind::Day, DAYS[0]).unwrap()[0];
    assert_eq!(story.status, StoryStatus::Accepted);
    assert!(
        stories::latest(&db, id, StoryKind::Day)
            .unwrap()
            .contains_key(DAYS[0])
    );
}

#[tokio::test]
async fn in_review_mode_background_entries_are_drafts_kept_out_of_the_journal() {
    let (db, id) = setup();
    review_mode(&db.lock().unwrap());
    queue::enqueue_finished(&db.lock().unwrap(), id, &journal(), TODAY).unwrap();
    let provider = FakeProvider::new([ENTRY]);
    let (_, on_event) = recorder();
    run_next(&db, &provider, &Control::new(false), "en", &on_event)
        .await
        .unwrap();
    let db = db.lock().unwrap();
    let story = &stories::versions(&db, id, StoryKind::Day, DAYS[0]).unwrap()[0];
    assert_eq!(story.status, StoryStatus::Draft);
    assert!(stories::latest(&db, id, StoryKind::Day).unwrap().is_empty());
    assert!(stories::day_entries(&db, id).unwrap().is_empty());
    assert!(stories::accepted(&db, id).unwrap().is_empty());
}

#[tokio::test]
async fn a_draft_asked_for_carries_its_note_even_in_automatic_mode() {
    let (db, id) = setup();
    let ask = Ask {
        narrator: Some("grumpy_dwarf"),
        draft: true,
        note: Some("shorter, please"),
        partial: false,
    };
    queue::request(&db.lock().unwrap(), id, StoryKind::Day, DAYS[0], &ask).unwrap();
    let provider = FakeProvider::new([ENTRY]);
    let (_, on_event) = recorder();
    run_next(&db, &provider, &Control::new(false), "en", &on_event)
        .await
        .unwrap();
    let db = db.lock().unwrap();
    let story = &stories::versions(&db, id, StoryKind::Day, DAYS[0]).unwrap()[0];
    assert_eq!(story.status, StoryStatus::Draft);
    assert_eq!(story.note.as_deref(), Some("shorter, please"));
    assert_eq!(story.narrator, "grumpy_dwarf");
    assert!(
        provider.requests()[0].user.contains("\"shorter, please\""),
        "the note reaches the model"
    );
}

#[tokio::test]
async fn a_day_with_a_draft_or_a_discarded_entry_is_never_queued_again() {
    let (db, id) = setup();
    review_mode(&db.lock().unwrap());
    queue::enqueue_finished(&db.lock().unwrap(), id, &journal(), TODAY).unwrap();
    let provider = FakeProvider::new([ENTRY, ENTRY]);
    let control = Control::new(false);
    let (_, on_event) = recorder();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    let db = db.lock().unwrap();
    let second = stories::versions(&db, id, StoryKind::Day, DAYS[1]).unwrap()[0].id;
    assert!(stories::discard(&db, id, second).unwrap());
    assert_eq!(
        queue::enqueue_finished(&db, id, &journal(), TODAY).unwrap(),
        0,
        "the drafted and the discarded day stay out of the queue"
    );
    let queued: Vec<String> = queue::jobs(&db, id)
        .unwrap()
        .into_iter()
        .map(|j| j.start_day)
        .collect();
    assert!(!queued.contains(&DAYS[0].to_string()));
    assert!(!queued.contains(&DAYS[1].to_string()));
}

#[test]
fn accepting_a_draft_puts_it_in_the_journal_and_sets_its_rivals_aside() {
    let (db, id) = setup();
    let db = db.lock().unwrap();
    let save = |text: &str, status| {
        stories::save(
            &db,
            &NewStory {
                character_id: id,
                kind: StoryKind::Day,
                start_day: DAYS[0].into(),
                end_day: DAYS[0].into(),
                narrator: "murloc_scribe".into(),
                language: "en".into(),
                provider: "fake".into(),
                model: "fake".into(),
                prompt_hash: "h".into(),
                text: text.into(),
                check: CheckStatus::Passed,
                check_reason: None,
                status,
                note: None,
                partial: false,
            },
        )
        .unwrap()
    };
    let kept = save("The first, accepted.", StoryStatus::Accepted);
    let one = save("A draft.", StoryStatus::Draft);
    let other = save("Another draft.", StoryStatus::Draft);

    assert!(stories::accept(&db, id, one).unwrap());
    let status = |sid: i64| {
        stories::all(&db, id)
            .unwrap()
            .into_iter()
            .find(|s| s.id == sid)
            .unwrap()
            .status
    };
    assert_eq!(status(one), StoryStatus::Accepted);
    assert_eq!(status(other), StoryStatus::Discarded);
    assert_eq!(status(kept), StoryStatus::Accepted, "older versions stay");
    assert_eq!(
        stories::latest(&db, id, StoryKind::Day).unwrap()[DAYS[0]].id,
        one
    );
    // Discarding only touches drafts: an accepted entry is not thrown away by mistake.
    assert!(!stories::discard(&db, id, kept).unwrap());
    // Another character's story is out of reach.
    assert!(!stories::accept(&db, id + 1, other).unwrap());
}

#[tokio::test]
async fn a_job_is_written_with_the_players_own_narrator() {
    let (db, id) = setup();
    let folder = tempfile::tempdir().unwrap();
    let mut mine = mrglglgl_core::narrators::copy_of("murloc_scribe", "en");
    mine.persona = "You are a heron who keeps this journal.".into();
    let mine = mrglglgl_core::narrators::save(folder.path(), mine).unwrap();
    let ask = Ask {
        narrator: Some(&mine.id),
        ..Ask::default()
    };
    queue::request(&db.lock().unwrap(), id, StoryKind::Day, DAYS[0], &ask).unwrap();
    let provider = FakeProvider::new([ENTRY]);
    let control = Control::new(false).with_narrators(folder.path().to_path_buf());
    let (_, on_event) = recorder();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    assert!(provider.requests()[0].system.contains("You are a heron"));
    let db = db.lock().unwrap();
    assert_eq!(
        stories::versions(&db, id, StoryKind::Day, DAYS[0]).unwrap()[0].narrator,
        mine.id
    );
}

fn heron() -> mrglglgl_core::narrators::Narrator {
    let mut n = mrglglgl_core::narrators::copy_of("murloc_scribe", "en");
    n.persona = "You are a heron who keeps this journal.".into();
    n
}

#[tokio::test]
async fn a_preview_writes_a_day_with_the_unsaved_narrator_and_stores_nothing() {
    let (db, id) = setup();
    let provider = FakeProvider::new([ENTRY]);
    let control = Control::new(false);
    let result = queue::preview(
        &db,
        &provider,
        &control,
        "en",
        &heron(),
        &PreviewDay::Character {
            character_id: id,
            day: DAYS[0].into(),
        },
        &|_| {},
    )
    .await
    .unwrap();
    assert!(matches!(result, WriteResult::Written { ref text, .. } if text == ENTRY));
    let request = &provider.requests()[0];
    assert!(request.system.contains("You are a heron"));
    assert_eq!(notes_day(request), "2026-11-02");
    let db = db.lock().unwrap();
    assert!(
        stories::all(&db, id).unwrap().is_empty(),
        "nothing is stored"
    );
    assert!(
        queue::jobs(&db, id).unwrap().is_empty(),
        "nothing is queued"
    );
}

#[tokio::test]
async fn a_preview_without_a_journal_writes_the_sample_day() {
    let (db, _) = setup();
    let provider = FakeProvider::new([ENTRY]);
    queue::preview(
        &db,
        &provider,
        &Control::new(false),
        "en",
        &heron(),
        &PreviewDay::Sample,
        &|_| {},
    )
    .await
    .unwrap();
    assert!(provider.requests()[0].user.contains("Prairie Wol"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_preview_can_be_cancelled() {
    let (db, id) = setup();
    let long = "word ".repeat(200);
    let provider =
        FakeProvider::new([long.as_str()]).with_pace(Pace::PerWord(Duration::from_millis(10)));
    let control = Arc::new(Control::new(false));
    let canceller = control.clone();
    let result = queue::preview(
        &db,
        &provider,
        &control,
        "en",
        &heron(),
        &PreviewDay::Character {
            character_id: id,
            day: DAYS[0].into(),
        },
        &move |_| {
            canceller.cancel_preview();
        },
    )
    .await
    .unwrap();
    assert_eq!(result, WriteResult::Cancelled);
}

#[tokio::test]
async fn a_day_told_so_far_is_shown_now_and_written_in_full_once_it_is_over() {
    let (db, id) = setup();
    review_mode(&db.lock().unwrap());
    let ask = Ask {
        partial: true,
        ..Ask::default()
    };
    queue::request(&db.lock().unwrap(), id, StoryKind::Day, TODAY, &ask).unwrap();
    let provider = FakeProvider::new([ENTRY, "The whole day, told at last."]);
    let control = Control::new(false);
    let (_, on_event) = recorder();
    run_next(&db, &provider, &control, "en", &on_event)
        .await
        .unwrap();
    assert!(
        provider.requests()[0].user.contains("not over"),
        "the writer is told the day goes on"
    );
    {
        let db = db.lock().unwrap();
        let so_far = &stories::latest(&db, id, StoryKind::Day).unwrap()[TODAY];
        assert!(so_far.partial);
        assert_eq!(
            so_far.status,
            StoryStatus::Accepted,
            "shown on the page at once, even in review mode"
        );
        assert!(
            !stories::day_entries(&db, id).unwrap().contains_key(TODAY),
            "a weekly chapter never reads a day told so far"
        );
        // The next day, the full entry is queued as if nothing had been written.
        queue::enqueue_finished(&db, id, &journal(), "2026-11-08").unwrap();
        assert!(
            queue::jobs(&db, id)
                .unwrap()
                .iter()
                .any(|j| j.start_day == TODAY && !j.partial)
        );
    }
}

#[test]
fn an_entry_edited_by_hand_is_a_new_version_the_fact_check_does_not_vouch_for() {
    let (db, id) = setup();
    let db = db.lock().unwrap();
    let save = |text: &str, status, check, partial| {
        stories::save(
            &db,
            &NewStory {
                character_id: id,
                kind: StoryKind::Day,
                start_day: DAYS[0].into(),
                end_day: DAYS[0].into(),
                narrator: "murloc_scribe".into(),
                language: "en".into(),
                provider: "fake".into(),
                model: "fake".into(),
                prompt_hash: "h".into(),
                text: text.into(),
                check,
                check_reason: None,
                status,
                note: None,
                partial,
            },
        )
        .unwrap()
    };
    let written = save(
        "The model's words.",
        StoryStatus::Accepted,
        CheckStatus::Passed,
        false,
    );
    let draft = save("A draft.", StoryStatus::Draft, CheckStatus::Passed, false);

    let edited = stories::edit(&db, id, written, "  My own words.\n\nTwo paragraphs.  ")
        .unwrap()
        .expect("an edit is saved");
    let all = stories::all(&db, id).unwrap();
    let get = |sid: i64| all.iter().find(|s| s.id == sid).unwrap().clone();
    let mine = get(edited);
    assert_eq!(mine.text, "My own words.\n\nTwo paragraphs.");
    assert_eq!(mine.check, CheckStatus::Edited);
    assert_eq!(mine.provider, stories::PLAYER);
    assert_eq!(mine.status, StoryStatus::Accepted);
    assert_eq!(mine.narrator, "murloc_scribe");
    assert_eq!(
        get(written).text,
        "The model's words.",
        "the old version is kept"
    );
    assert_eq!(get(draft).status, StoryStatus::Discarded);
    assert_eq!(
        stories::latest(&db, id, StoryKind::Day).unwrap()[DAYS[0]].id,
        edited,
        "the journal shows the edit"
    );

    // A day set aside by the fact check can be fixed by hand, and then shows the text.
    let failed = save(
        "Invented things.",
        StoryStatus::Accepted,
        CheckStatus::Failed,
        false,
    );
    let fixed = stories::edit(&db, id, failed, "Only what happened.")
        .unwrap()
        .unwrap();
    assert_eq!(
        stories::day_entries(&db, id).unwrap()[DAYS[0]],
        "Only what happened."
    );
    assert!(fixed > failed);

    // Refused: blank text, another character's entry, a day told only so far.
    assert_eq!(stories::edit(&db, id, written, "  \n ").unwrap(), None);
    assert_eq!(stories::edit(&db, id + 1, written, "Mine.").unwrap(), None);
    let so_far = save("So far.", StoryStatus::Accepted, CheckStatus::Passed, true);
    assert_eq!(stories::edit(&db, id, so_far, "Mine.").unwrap(), None);
}

#[test]
fn every_characters_jobs_are_listed_in_the_order_they_run() {
    let (db, wren) = setup();
    let db = db.into_inner().unwrap();
    let brakka = db
        .upsert_character(
            &CharacterSource {
                flavor: "_forever_".into(),
                account: "ACCOUNT1".into(),
                realm_folder: "93".into(),
                character_folder: "Brakka".into(),
                journal_path: PathBuf::from("/wow/Brakka.lua"),
            },
            None,
            None,
        )
        .unwrap();
    archive::merge_snapshot(&db, brakka, &journal(), 1).unwrap();
    queue::request(&db, wren, StoryKind::Day, DAYS[0], &Ask::default()).unwrap();
    queue::request(&db, brakka, StoryKind::Day, DAYS[1], &Ask::default()).unwrap();

    let all = queue::all_jobs(&db).unwrap();
    let who: Vec<(i64, Option<u32>)> = all.iter().map(|j| (j.character_id, j.position)).collect();
    assert_eq!(who, vec![(wren, Some(1)), (brakka, Some(2))]);
    // A character's own list keeps the shared positions.
    assert_eq!(queue::jobs(&db, brakka).unwrap()[0].position, Some(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_preview_waiting_for_its_turn_shows_and_can_be_stopped() {
    let (db, id) = setup();
    let db = Arc::new(db);
    queue::request(
        &db.lock().unwrap(),
        id,
        StoryKind::Day,
        DAYS[0],
        &Ask::default(),
    )
    .unwrap();
    let slow = "word ".repeat(100);
    let provider = Arc::new(
        FakeProvider::new([slow.as_str()]).with_pace(Pace::PerWord(Duration::from_millis(20))),
    );
    let control = Arc::new(Control::new(false));

    // An entry takes the writer.
    let (entry_db, entry_provider, entry_control) = (db.clone(), provider.clone(), control.clone());
    let entry = tokio::spawn(async move {
        run_next(
            &entry_db,
            entry_provider.as_ref(),
            &entry_control,
            "en",
            &|_| {},
        )
        .await
    });
    while control.current().is_none() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // A preview asked meanwhile waits, and says so.
    let (preview_db, preview_provider, preview_control) =
        (db.clone(), provider.clone(), control.clone());
    let preview = tokio::spawn(async move {
        queue::preview(
            &preview_db,
            preview_provider.as_ref(),
            &preview_control,
            "en",
            &heron(),
            &PreviewDay::Sample,
            &|_| {},
        )
        .await
    });
    while control.preview_status().is_none() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let status = control.preview_status().unwrap();
    assert!(!status.running);
    assert_eq!(status.narrator, heron().name);

    // Stopping it does not wait for the entry to finish.
    assert!(control.cancel_preview());
    let result = tokio::time::timeout(Duration::from_millis(500), preview)
        .await
        .expect("a waiting preview stops at once")
        .unwrap()
        .unwrap();
    assert_eq!(result, WriteResult::Cancelled);
    assert!(control.preview_status().is_none());
    entry.await.unwrap().unwrap();
}

fn story(id: i64, day: &str, status: StoryStatus, partial: bool) -> NewStory {
    NewStory {
        character_id: id,
        kind: StoryKind::Day,
        start_day: day.into(),
        end_day: day.into(),
        narrator: "murloc_scribe".into(),
        language: "en".into(),
        provider: "fake".into(),
        model: "fake".into(),
        prompt_hash: "h".into(),
        text: "Words.".into(),
        check: CheckStatus::Passed,
        check_reason: None,
        status,
        note: None,
        partial,
    }
}

#[test]
fn a_newer_entry_in_the_journal_sets_older_drafts_of_its_day_aside() {
    let (db, id) = setup();
    let db = db.lock().unwrap();
    let old = stories::save(&db, &story(id, DAYS[0], StoryStatus::Draft, false)).unwrap();
    let elsewhere = stories::save(&db, &story(id, DAYS[1], StoryStatus::Draft, false)).unwrap();
    assert!(stories::draft_waiting(&db, id, StoryKind::Day, DAYS[0]).unwrap());

    // A day told only so far is not the day's entry: the draft still waits.
    stories::save(&db, &story(id, DAYS[0], StoryStatus::Accepted, true)).unwrap();
    assert!(stories::draft_waiting(&db, id, StoryKind::Day, DAYS[0]).unwrap());

    // The full entry goes in: the older draft could only ever replace it with less.
    stories::save(&db, &story(id, DAYS[0], StoryStatus::Accepted, false)).unwrap();
    let status = |sid: i64| {
        stories::all(&db, id)
            .unwrap()
            .into_iter()
            .find(|s| s.id == sid)
            .unwrap()
            .status
    };
    assert_eq!(status(old), StoryStatus::Discarded);
    assert!(!stories::draft_waiting(&db, id, StoryKind::Day, DAYS[0]).unwrap());
    assert_eq!(
        status(elsewhere),
        StoryStatus::Draft,
        "other days keep theirs"
    );
}
