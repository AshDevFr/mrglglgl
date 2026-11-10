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

/// A server that lists `models` and answers a chat completion only for `answers`.
fn endpoint(models: &'static str, answers: &'static str) -> impl Fn(Request) -> Response {
    move |req: Request| {
        if req.header("authorization") != Some("Bearer secret") {
            return Response::new(401, "no");
        }
        match req.path.as_str() {
            "/v1/models" => Response::new(200, format!(r#"{{"data": [{models}]}}"#)),
            "/v1/chat/completions" => {
                let body: serde_json::Value = serde_json::from_str(&req.body).unwrap();
                // A check, not an entry: it asks for next to nothing.
                assert!(body["max_tokens"].as_u64().unwrap() <= 16);
                if body["model"] == answers {
                    Response::new(200, r#"{"choices": [{"message": {"content": "Hi"}}]}"#)
                } else {
                    Response::new(
                        400,
                        r#"{"error": {"message": "Invalid model name passed in model=Qq"}}"#,
                    )
                }
            }
            other => panic!("unexpected {other}"),
        }
    }
}

fn with_model(url: &str, model: &str) -> EndpointProvider {
    EndpointProvider::new(EndpointConfig {
        base_url: format!("{url}/v1"),
        model: model.into(),
        api_key: Some("secret".into()),
    })
}

#[tokio::test]
async fn test_connection_lists_the_models_and_asks_the_chosen_one() {
    let server = serve(endpoint(
        r#"{"id": "scribe-3b"}, {"id": "a-big-one"}"#,
        "scribe-3b",
    ))
    .await;
    // No model yet: the test lists what there is to choose from, in order.
    assert_eq!(
        with_model(&server.url, "").test_connection().await,
        Ok(vec!["a-big-one".to_string(), "scribe-3b".to_string()])
    );
    assert_eq!(
        provider(&server.url, Some("secret"))
            .test_connection()
            .await
            .map(|m| m.len()),
        Ok(2)
    );
    assert_eq!(
        provider(&server.url, Some("wrong")).test_connection().await,
        Err(ProviderError::Unauthorised)
    );
    assert_eq!(
        provider("http://127.0.0.1:9", None).test_connection().await,
        Err(ProviderError::Unreachable("127.0.0.1".into()))
    );
}

#[tokio::test]
async fn test_connection_says_no_to_a_model_that_does_not_answer() {
    // Listed or not, a model is only fine once it answers: some servers list nothing useful.
    let unlisted = serve(endpoint("", "scribe-3b")).await;
    assert_eq!(
        with_model(&unlisted.url, "Qq").test_connection().await,
        Err(ProviderError::ModelNotFound("Qq".into()))
    );
    assert_eq!(
        with_model(&unlisted.url, "scribe-3b")
            .test_connection()
            .await,
        Ok(vec![])
    );
    let listed = serve(endpoint(r#"{"id": "other"}"#, "other")).await;
    assert_eq!(
        with_model(&listed.url, "scribe-3b").test_connection().await,
        Err(ProviderError::ModelNotFound("scribe-3b".into()))
    );
}

/// What a writer gets from a stream that carries `events` and then ends.
async fn streamed(events: &[serde_json::Value]) -> Result<String, ProviderError> {
    let mut chunks: Vec<Vec<u8>> = events
        .iter()
        .map(|e| format!("data: {e}\n\n").into_bytes())
        .collect();
    chunks.push(b"data: [DONE]\n\n".to_vec());
    let server = serve(move |_| {
        Response::chunked(200, chunks.clone(), Duration::from_millis(1))
            .header("Content-Type", "text/event-stream")
    })
    .await;
    generate_text(
        &provider(&server.url, None),
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        None,
    )
    .await
    .map(|(_, text)| text)
}

#[tokio::test]
async fn an_error_inside_the_stream_is_an_error() {
    // A proxy such as LiteLLM answers 200 and then reports its backend's failure in the stream.
    let result = streamed(&[serde_json::json!({
        "error": {"message": "backend model is not loaded"}
    })])
    .await;
    match result {
        Err(ProviderError::Failed(message)) => {
            assert!(message.contains("backend model is not loaded"), "{message}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test]
async fn a_stream_with_no_text_is_an_error_not_an_empty_entry() {
    match streamed(&[]).await {
        Err(ProviderError::Failed(message)) => assert!(message.contains("no text"), "{message}"),
        other => panic!("expected a failure, got {other:?}"),
    }
    // A thinking model that spends every token thinking writes nothing either, and says why.
    let thinking = streamed(&[
        serde_json::json!({"choices": [{"delta": {"reasoning_content": "Let me think about"}}]}),
        serde_json::json!({"choices": [{"delta": {"reasoning_content": " the fish."}, "finish_reason": "length"}]}),
    ])
    .await;
    match thinking {
        Err(ProviderError::Failed(message)) => assert!(message.contains("thinking"), "{message}"),
        other => panic!("expected a failure, got {other:?}"),
    }
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

/// A server that, like OpenAI's newer models, refuses `max_tokens` and any temperature but the
/// default, and records each body it was sent.
fn newer_openai(bodies: Arc<Mutex<Vec<serde_json::Value>>>) -> impl Fn(Request) -> Response {
    move |req: Request| {
        if req.path == "/v1/models" {
            return Response::new(200, r#"{"data": [{"id": "scribe-3b"}]}"#);
        }
        let body: serde_json::Value = serde_json::from_str(&req.body).unwrap();
        bodies.lock().unwrap().push(body.clone());
        if body.get("max_tokens").is_some() {
            return Response::new(
                400,
                r#"{"error": {"message": "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.", "type": "invalid_request_error", "param": "max_tokens", "code": "unsupported_parameter"}}"#,
            );
        }
        if body.get("temperature").is_some() {
            return Response::new(
                400,
                r#"{"error": {"message": "Unsupported value: 'temperature' does not support 0.4 with this model. Only the default (1) value is supported.", "type": "invalid_request_error", "param": "temperature", "code": "unsupported_value"}}"#,
            );
        }
        if body["stream"] == true {
            Response::chunked(
                200,
                sse(&["Caught", " six", " fish."]),
                Duration::from_millis(1),
            )
            .header("Content-Type", "text/event-stream")
        } else {
            Response::new(200, r#"{"choices": [{"message": {"content": ""}}]}"#)
        }
    }
}

#[tokio::test]
async fn a_newer_model_that_wants_max_completion_tokens_and_no_temperature_still_writes() {
    let bodies: Arc<Mutex<Vec<serde_json::Value>>> = Arc::default();
    let server = serve(newer_openai(bodies.clone())).await;
    let provider = provider(&server.url, None);
    let (_, text) = generate_text(
        &provider,
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(text, "Caught six fish.");
    let sent = bodies.lock().unwrap().clone();
    let last = sent.last().unwrap();
    assert_eq!(last["max_completion_tokens"], 200);
    assert!(last.get("max_tokens").is_none());
    assert!(last.get("temperature").is_none());

    // What it learned holds for the next entry: no refused request first.
    bodies.lock().unwrap().clear();
    generate_text(
        &provider,
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(bodies.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn the_connection_test_is_not_fooled_into_calling_a_newer_model_missing() {
    let server = serve(newer_openai(Arc::default())).await;
    assert_eq!(
        provider(&server.url, None).test_connection().await,
        Ok(vec!["scribe-3b".to_string()])
    );
}

#[tokio::test]
async fn a_sign_in_page_in_front_of_the_api_is_named_not_read_as_empty() {
    // A proxy such as a single sign-on gate redirects every API call to its login page, which
    // answers 200 with HTML: that is neither "no models" nor "no text".
    let gate = serve(|req: Request| match req.path.as_str() {
        "/login" => Response::new(200, "<html>Sign in</html>")
            .header("Content-Type", "text/html; charset=utf-8"),
        _ => Response::new(302, "").header("Location", "/login"),
    })
    .await;
    let sign_in =
        serve(|_| Response::new(200, "<html>Sign in</html>").header("Content-Type", "text/html"))
            .await;
    let elsewhere = sign_in.url.replace("127.0.0.1", "localhost");
    let redirecting = serve(move |_: Request| {
        Response::new(302, "").header("Location", &format!("{elsewhere}/authorize"))
    })
    .await;

    let same_host = with_model(&gate.url, "").test_connection().await;
    assert_eq!(
        same_host,
        Err(ProviderError::Failed(
            "127.0.0.1 answered with a web page, not the API".into()
        ))
    );
    let Err(ProviderError::Failed(message)) =
        with_model(&redirecting.url, "").test_connection().await
    else {
        panic!("a redirect to a sign-in page passed the test");
    };
    assert!(message.contains("web page at localhost"), "{message}");
    // Writing names it too, instead of saying the model sent no text.
    let written = generate_text(
        &with_model(&gate.url, "scribe-3b"),
        request(),
        Duration::from_secs(5),
        CancellationToken::new(),
        None,
    )
    .await;
    assert_eq!(
        written.map(|(_, text)| text),
        Err(ProviderError::Failed(
            "127.0.0.1 answered with a web page, not the API".into()
        ))
    );
}

#[tokio::test]
async fn models_are_read_from_the_shapes_servers_send() {
    for body in [
        r#"{"object": "list", "data": [{"id": "b"}, {"id": "a"}]}"#,
        r#"[{"id": "b"}, "a"]"#,
        r#"{"models": [{"name": "b"}, {"model": "a"}]}"#,
    ] {
        let server = serve(move |_| Response::new(200, body)).await;
        assert_eq!(
            with_model(&server.url, "").list_models().await,
            Ok(vec!["a".to_string(), "b".to_string()]),
            "{body}"
        );
    }
    let nothing_there = serve(|_| Response::new(404, "not found")).await;
    let Err(ProviderError::Failed(message)) =
        with_model(&nothing_there.url, "").list_models().await
    else {
        panic!("a missing list read as an empty one");
    };
    assert!(
        message.ends_with("/v1/models answered 404: no model list there"),
        "{message}"
    );
}
