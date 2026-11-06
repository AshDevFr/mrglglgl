use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mrglglgl_core::archive;
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::CharacterSource;
use mrglglgl_core::game::is_game_process;
use mrglglgl_core::queue::{
    self, CANCELLED, Control, JobState, MAX_ATTEMPTS, QueueEvent, drain, run_next,
};
use mrglglgl_core::schema::{Character, Day, Event, EventKind, Journal};
use mrglglgl_core::stories::{self, StoryKind};
use mrglglgl_core::writer::CheckStatus;
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
    queue::request(&db.lock().unwrap(), id, StoryKind::Day, DAYS[0], None).unwrap();
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
    queue::request(&db.lock().unwrap(), id, StoryKind::Day, DAYS[0], None).unwrap();
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
        Some("grumpy_dwarf"),
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
    queue::request(&db.lock().unwrap(), id, StoryKind::Day, DAYS[0], None).unwrap();
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
