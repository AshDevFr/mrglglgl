mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{Request, Response, serve};
use mrglglgl_llm::endpoint::{EndpointConfig, EndpointProvider, sends_off_machine};
use mrglglgl_llm::{
    CancellationToken, GenerateOutcome, GenerateRequest, ProviderError, generate, generate_text,
};

fn request() -> GenerateRequest {
    GenerateRequest {
        system: "You are the murloc scribe.".into(),
        user: "Notes: caught 6 fish.".into(),
        max_tokens: 200,
        temperature: 0.4,
        language: "en".into(),
    }
}

fn provider(url: &str, key: Option<&str>) -> EndpointProvider {
    EndpointProvider::new(EndpointConfig {
        base_url: format!("{url}/v1"),
        model: "scribe-3b".into(),
        api_key: key.map(str::to_string),
    })
}

fn sse(words: &[&str]) -> Vec<Vec<u8>> {
    let mut chunks: Vec<Vec<u8>> = words
        .iter()
        .map(|w| {
            let event = serde_json::json!({"choices": [{"delta": {"content": w}}]});
            format!("data: {event}\n\n").into_bytes()
        })
        .collect();
    chunks.push(b"data: [DONE]\n\n".to_vec());
    chunks
}

#[tokio::test]
async fn streams_a_chat_completion() {
    let seen: Arc<Mutex<Option<Request>>> = Arc::default();
    let log = seen.clone();
    let server = serve(move |req| {
        *log.lock().unwrap() = Some(req);
        Response::chunked(
            200,
            sse(&["Caught", " six", " fish."]),
            Duration::from_millis(5),
        )
        .header("Content-Type", "text/event-stream")
    })
    .await;
    let (outcome, text) = generate_text(
        &provider(&server.url, Some("secret")),
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::Completed);
    assert_eq!(text, "Caught six fish.");

    let req = seen.lock().unwrap().clone().unwrap();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/v1/chat/completions");
    assert_eq!(req.header("authorization"), Some("Bearer secret"));
    let body: serde_json::Value = serde_json::from_str(&req.body).unwrap();
    assert_eq!(body["model"], "scribe-3b");
    assert_eq!(body["stream"], true);
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][1]["content"], "Notes: caught 6 fish.");
}

#[tokio::test]
async fn an_event_split_across_reads_is_put_back_together() {
    let whole = sse(&["Mrgl."]).concat();
    let (a, b) = whole.split_at(17);
    let (a, b) = (a.to_vec(), b.to_vec());
    let server = serve(move |_| {
        Response::chunked(200, vec![a.clone(), b.clone()], Duration::from_millis(20))
    })
    .await;
    let (_, text) = generate_text(
        &provider(&server.url, None),
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(text, "Mrgl.");
}

async fn slow_server() -> common::MockServer {
    let words: Vec<&str> = std::iter::repeat_n(" fish", 50).collect();
    serve(move |_| Response::chunked(200, sse(&words), Duration::from_millis(200))).await
}

#[tokio::test]
async fn cancel_stops_the_stream_within_two_seconds() {
    let server = slow_server().await;
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        trigger.cancel();
    });
    let started = Instant::now();
    let (outcome, text) = generate_text(
        &provider(&server.url, None),
        request(),
        Duration::from_secs(120),
        cancel,
        None,
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!text.is_empty() && text.len() < 50 * 5);
}

#[tokio::test]
async fn a_slow_endpoint_times_out() {
    let server = slow_server().await;
    let started = Instant::now();
    let outcome = generate(
        &provider(&server.url, None),
        request(),
        Duration::from_millis(400),
        CancellationToken::new(),
        tokio::sync::mpsc::channel(64).0,
    )
    .await
    .unwrap();
    assert_eq!(outcome, GenerateOutcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn errors_are_named_plainly() {
    let refused = serve(|_| Response::new(401, r#"{"error":"bad key"}"#)).await;
    let missing = serve(|_| Response::new(404, r#"{"error":"model not found"}"#)).await;
    let ollama_style = serve(|_| {
        Response::new(
            400,
            r#"{"error":"model \"scribe-3b\" not found, try pulling it first"}"#,
        )
    })
    .await;
    let run = |url: String| async move {
        generate(
            &provider(&url, Some("wrong")),
            request(),
            Duration::from_secs(5),
            CancellationToken::new(),
            tokio::sync::mpsc::channel(4).0,
        )
        .await
    };
    assert_eq!(
        run(refused.url.clone()).await,
        Err(ProviderError::Unauthorised)
    );
    assert_eq!(
        run(missing.url.clone()).await,
        Err(ProviderError::ModelNotFound("scribe-3b".into()))
    );
    assert_eq!(
        run(ollama_style.url.clone()).await,
        Err(ProviderError::ModelNotFound("scribe-3b".into()))
    );
    // Nothing listens on port 9 on a test machine.
    assert_eq!(
        run("http://127.0.0.1:9".into()).await,
        Err(ProviderError::Unreachable("127.0.0.1".into()))
    );
    assert_eq!(
        ProviderError::Unreachable("127.0.0.1".into()).to_string(),
        "could not reach 127.0.0.1"
    );
}

#[tokio::test]
async fn test_connection_checks_the_key_and_the_model() {
    let listing = |models: &'static str| {
        move |req: Request| {
            assert_eq!(req.path, "/v1/models");
            if req.header("authorization") != Some("Bearer secret") {
                return Response::new(401, "no");
            }
            Response::new(200, format!(r#"{{"data": [{models}]}}"#))
        }
    };
    let has = serve(listing(r#"{"id": "scribe-3b"}"#)).await;
    let lacks = serve(listing(r#"{"id": "other"}"#)).await;
    assert_eq!(
        provider(&has.url, Some("secret")).test_connection().await,
        Ok(())
    );
    assert_eq!(
        provider(&has.url, Some("wrong")).test_connection().await,
        Err(ProviderError::Unauthorised)
    );
    assert_eq!(
        provider(&lacks.url, Some("secret")).test_connection().await,
        Err(ProviderError::ModelNotFound("scribe-3b".into()))
    );
    assert_eq!(
        provider("http://127.0.0.1:9", None).test_connection().await,
        Err(ProviderError::Unreachable("127.0.0.1".into()))
    );
}

#[test]
fn only_loopback_addresses_stay_on_the_machine() {
    for local in [
        "http://localhost:11434/v1",
        "http://127.0.0.1:1234/v1",
        "http://[::1]:8080/v1",
        "http://LOCALHOST/v1",
    ] {
        assert!(!sends_off_machine(local), "{local}");
    }
    for remote in [
        "https://api.example.com/v1",
        "http://192.168.1.20:11434/v1",
        "http://gpu-box.local:8080/v1",
        "not a url",
    ] {
        assert!(sends_off_machine(remote), "{remote}");
    }
}
