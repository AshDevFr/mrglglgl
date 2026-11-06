use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use mrglglgl_llm::fake::{FakeProvider, Pace};
use mrglglgl_llm::{
    CancellationToken, GenerateOutcome, GenerateRequest, ProviderError, generate, generate_text,
};
use tokio::sync::mpsc;

fn request() -> GenerateRequest {
    GenerateRequest {
        system: "You are the murloc scribe.".into(),
        user: "Notes: caught 6 fish.".into(),
        max_tokens: 200,
        temperature: 0.4,
        language: "en".into(),
    }
}

const LONG_REPLY: &str = "The pond was calm and the fish were generous and the light lay \
     low over the water all afternoon while I waited for one more bite";

#[tokio::test]
async fn text_streams_in_order() {
    let provider = FakeProvider::new(["Caught six fish today."])
        .with_pace(Pace::PerWord(Duration::from_millis(1)));
    let (tx, mut rx) = mpsc::channel(16);
    let (outcome, text) = generate_text(
        &provider,
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        Some(tx),
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::Completed);
    assert_eq!(text, "Caught six fish today.");
    let mut chunks = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        chunks.push(chunk);
    }
    assert_eq!(chunks, vec!["Caught", " six", " fish", " today."]);
    assert_eq!(provider.requests(), vec![request()]);
}

#[tokio::test]
async fn cancel_stops_within_two_seconds() {
    let provider =
        FakeProvider::new([LONG_REPLY]).with_pace(Pace::PerWord(Duration::from_millis(200)));
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        trigger.cancel();
    });
    let started = Instant::now();
    let (outcome, text) =
        generate_text(&provider, request(), Duration::from_secs(60), cancel, None)
            .await
            .unwrap();
    assert_eq!(outcome, GenerateOutcome::Cancelled);
    assert!(
        started.elapsed() < Duration::from_millis(2300),
        "{:?}",
        started.elapsed()
    );
    assert!(text.len() < LONG_REPLY.len(), "stopped part-way");
}

#[tokio::test]
async fn a_timeout_is_reported_as_such() {
    let provider =
        FakeProvider::new([LONG_REPLY]).with_pace(Pace::PerWord(Duration::from_millis(200)));
    let started = Instant::now();
    let outcome = generate(
        &provider,
        request(),
        Duration::from_millis(300),
        CancellationToken::new(),
        mpsc::channel(64).0,
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::TimedOut);
    assert!(started.elapsed() < Duration::from_millis(2300));
}

#[tokio::test]
async fn a_provider_slow_to_notice_is_given_up_on_promptly() {
    // Stuck for ten seconds without looking at the token, like a long prompt evaluation.
    let provider = FakeProvider::new(["late"]).with_pace(Pace::Stubborn(Duration::from_secs(10)));
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        trigger.cancel();
    });
    let started = Instant::now();
    let outcome = generate(
        &provider,
        request(),
        Duration::from_secs(60),
        cancel,
        mpsc::channel(4).0,
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::Cancelled);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn errors_pass_through() {
    let provider = FakeProvider::scripted([Err(ProviderError::Unavailable("no model".into()))]);
    let result = generate(
        &provider,
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        mpsc::channel(4).0,
    )
    .await;
    assert_eq!(result, Err(ProviderError::Unavailable("no model".into())));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_runtime_keeps_answering_while_a_generation_runs() {
    // The busy provider holds a blocking thread for a second, as inference does; the async side
    // (where the window's commands run) must keep ticking meanwhile.
    let provider = FakeProvider::new(["done"]).with_pace(Pace::Stubborn(Duration::from_secs(1)));
    let ticks = Arc::new(AtomicU32::new(0));
    let ticker = {
        let ticks = ticks.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(20)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        })
    };
    let outcome = generate(
        &provider,
        request(),
        Duration::from_secs(10),
        CancellationToken::new(),
        mpsc::channel(4).0,
    )
    .await
    .unwrap();
    ticker.abort();
    assert_eq!(outcome, GenerateOutcome::Completed);
    assert!(
        ticks.load(Ordering::SeqCst) >= 20,
        "{} ticks",
        ticks.load(Ordering::SeqCst)
    );
}

/// Runs against a real GGUF only when `MRGLGLGL_TEST_MODEL` points at one, since models are
/// gigabytes and never part of the repository; skipped otherwise. Set
/// `MRGLGLGL_TEST_MODEL_REASONING=1` for models with a reasoning mode (Qwen3.5, SmolLM3).
#[cfg(feature = "local")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_real_local_model_writes_and_can_be_cancelled() {
    use mrglglgl_llm::local::{LocalOptions, LocalProvider};

    let Some(path) = std::env::var_os("MRGLGLGL_TEST_MODEL") else {
        eprintln!("skipped: set MRGLGLGL_TEST_MODEL to a GGUF file to run this test");
        return;
    };
    let reasoning = std::env::var_os("MRGLGLGL_TEST_MODEL_REASONING").is_some();
    let provider = LocalProvider::new(LocalOptions::new(path, "test", reasoning));

    let (outcome, text) = generate_text(
        &provider,
        GenerateRequest {
            max_tokens: 60,
            ..request()
        },
        Duration::from_secs(300),
        CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::Completed);
    assert!(!text.trim().is_empty());
    assert!(!text.contains("<think>"), "reasoning stayed off: {text}");
    eprintln!("model wrote: {text}");

    // The model is loaded now, so cancelling mid-stream measures generation, not loading.
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let (tx, mut rx) = mpsc::channel(64);
    let run = tokio::spawn(async move {
        generate(
            &provider,
            GenerateRequest {
                max_tokens: 400,
                ..request()
            },
            Duration::from_secs(300),
            cancel,
            tx,
        )
        .await
    });
    rx.recv().await.expect("a first chunk");
    let asked = Instant::now();
    trigger.cancel();
    let outcome = run.await.unwrap().unwrap();
    assert_eq!(outcome, GenerateOutcome::Cancelled);
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "{:?}",
        asked.elapsed()
    );
}
