mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{Response, serve};
use mrglglgl_llm::CancellationToken;
use mrglglgl_llm::catalogue::{
    self, Advice, Catalogue, DownloadError, ModelEntry, Progress, Shortfall,
};

const GIB: u64 = 1 << 30;

#[test]
fn the_shipped_list_has_three_models_and_recommends_qwen() {
    let c = Catalogue::built_in();
    let ids: Vec<&str> = c.models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, vec!["qwen3.5-4b", "ministral-3-3b", "smollm3-3b"]);
    assert_eq!(c.recommended("en").unwrap().id, "qwen3.5-4b");
    assert_eq!(c.recommended("fr").unwrap().id, "qwen3.5-4b");
    assert_eq!(
        c.recommended("de").unwrap().id,
        "qwen3.5-4b",
        "overall recommendation"
    );
    let qwen = c.get("qwen3.5-4b").unwrap();
    assert_eq!(
        qwen.url(catalogue::HUGGING_FACE),
        "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-Q4_K_M.gguf"
    );
    assert!(qwen.reasoning);
    assert!(!c.get("ministral-3-3b").unwrap().reasoning);
    assert!(c.get("smollm3-3b").unwrap().reasoning);
    for m in &c.models {
        assert_eq!(m.licence, "Apache-2.0");
        assert!(m.prose_note.contains_key("en") && m.prose_note.contains_key("fr"));
    }
}

#[test]
fn a_newer_list_in_the_data_folder_replaces_the_built_in_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalogue.json");
    let mut newer = Catalogue::built_in();
    newer.version += 1;
    newer.models.truncate(1);
    std::fs::write(&path, serde_json::to_string(&newer).unwrap()).unwrap();
    assert_eq!(Catalogue::load(Some(&path)), newer);

    let mut older = Catalogue::built_in();
    older.models.truncate(1);
    std::fs::write(&path, serde_json::to_string(&older).unwrap()).unwrap();
    assert_eq!(Catalogue::load(Some(&path)), Catalogue::built_in());

    std::fs::write(&path, "{ not json").unwrap();
    assert_eq!(Catalogue::load(Some(&path)), Catalogue::built_in());
    assert_eq!(Catalogue::load(None), Catalogue::built_in());
}

#[test]
fn first_run_recommends_a_model_that_fits() {
    let c = Catalogue::built_in();
    assert_eq!(
        c.advise(16 * GIB, 100 * GIB, "en"),
        Advice::Recommend {
            id: "qwen3.5-4b".into()
        }
    );
    // An "8 GB" machine reports a little less and still counts.
    assert_eq!(
        c.advise(7_800_000_000, 100 * GIB, "fr"),
        Advice::Recommend {
            id: "qwen3.5-4b".into()
        }
    );
    // Too little disk for the recommendation: the smallest model that fits.
    assert_eq!(
        c.advise(16 * GIB, 3 * GIB, "en"),
        Advice::Recommend {
            id: "smollm3-3b".into()
        }
    );
}

#[test]
fn below_the_floor_it_says_why_and_downloads_nothing() {
    let c = Catalogue::built_in();
    assert_eq!(
        c.advise(4 * GIB, 100 * GIB, "en"),
        Advice::BelowFloor {
            reason: Shortfall::Memory {
                needed_gb: 8,
                has_gb: 4.0
            }
        }
    );
    match c.advise(16 * GIB, 2 * GIB, "en") {
        Advice::BelowFloor {
            reason: Shortfall::Disk { needed_gb, free_gb },
        } => {
            assert!(needed_gb > 2.7 && needed_gb < 3.0, "{needed_gb}");
            assert_eq!(free_gb, 2.0);
        }
        other => panic!("{other:?}"),
    }
}

const BODY_LEN: usize = 4000;

fn body() -> Vec<u8> {
    (0..BODY_LEN).map(|i| (i % 251) as u8).collect()
}

fn entry() -> ModelEntry {
    ModelEntry {
        id: "tiny".into(),
        name: "Tiny".into(),
        repo: "someone/tiny-GGUF".into(),
        file: "tiny.gguf".into(),
        size_bytes: BODY_LEN as u64,
        licence: "Apache-2.0".into(),
        languages: vec!["en".into()],
        reasoning: false,
        min_ram_gb: 1,
        recommended: true,
        recommended_for: vec![],
        prose_note: Default::default(),
    }
}

/// Serves the body in 10 chunks, honouring `Range: bytes=N-` with a 206.
async fn model_server(pause: Duration) -> common::MockServer {
    serve(move |req| {
        assert_eq!(req.path, "/someone/tiny-GGUF/resolve/main/tiny.gguf");
        let start: usize = req
            .header("range")
            .and_then(|r| r.strip_prefix("bytes="))
            .and_then(|r| r.trim_end_matches('-').parse().ok())
            .unwrap_or(0);
        let rest = body()[start..].to_vec();
        let chunks: Vec<Vec<u8>> = rest.chunks(BODY_LEN / 10).map(<[u8]>::to_vec).collect();
        Response::chunked(if start > 0 { 206 } else { 200 }, chunks, pause)
    })
    .await
}

#[tokio::test]
async fn a_model_downloads_with_progress_and_is_not_downloaded_twice() {
    let server = model_server(Duration::ZERO).await;
    let dir = tempfile::tempdir().unwrap();
    let seen = Arc::new(Mutex::new(Vec::<Progress>::new()));
    let log = seen.clone();
    let path = catalogue::download(
        &entry(),
        dir.path(),
        &server.url,
        Some(100 * GIB),
        &CancellationToken::new(),
        move |p| log.lock().unwrap().push(p),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), body());
    assert!(entry().installed(dir.path()));
    let seen = seen.lock().unwrap().clone();
    assert!(seen.windows(2).all(|w| w[0].downloaded <= w[1].downloaded));
    assert_eq!(seen.last().unwrap().downloaded, BODY_LEN as u64);

    let hits = server.hits();
    catalogue::download(
        &entry(),
        dir.path(),
        &server.url,
        None,
        &CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(server.hits(), hits, "the cached model is used");

    catalogue::delete(&entry(), dir.path()).unwrap();
    assert!(!entry().installed(dir.path()));
}

#[tokio::test]
async fn a_cancelled_download_stops_and_resumes_where_it_left_off() {
    let server = model_server(Duration::from_millis(100)).await;
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let result = catalogue::download(&entry(), dir.path(), &server.url, None, &cancel, move |p| {
        if p.downloaded > 0 {
            trigger.cancel();
        }
    })
    .await;
    assert_eq!(result, Err(DownloadError::Cancelled));
    assert!(!entry().installed(dir.path()));
    let kept = entry().partial_bytes(dir.path());
    assert!(kept > 0 && kept < BODY_LEN as u64, "{kept}");

    let path = catalogue::download(
        &entry(),
        dir.path(),
        &server.url,
        None,
        &CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(path).unwrap(),
        body(),
        "resumed, not duplicated"
    );
}

#[tokio::test]
async fn too_little_disk_space_refuses_before_downloading() {
    let server = model_server(Duration::ZERO).await;
    let dir = tempfile::tempdir().unwrap();
    let result = catalogue::download(
        &entry(),
        dir.path(),
        &server.url,
        Some(1000),
        &CancellationToken::new(),
        |_| {},
    )
    .await;
    assert!(matches!(result, Err(DownloadError::NoSpace { .. })));
    assert_eq!(server.hits(), 0);
}

#[tokio::test]
async fn a_truncated_download_is_not_taken_for_a_model() {
    let server = serve(|_| Response::new(200, vec![1u8; 10])).await;
    let dir = tempfile::tempdir().unwrap();
    let result = catalogue::download(
        &entry(),
        dir.path(),
        &server.url,
        None,
        &CancellationToken::new(),
        |_| {},
    )
    .await;
    assert_eq!(
        result,
        Err(DownloadError::WrongSize {
            got: 10,
            expected: BODY_LEN as u64
        })
    );
    assert!(!entry().installed(dir.path()));
    let missing = serve(|_| Response::new(404, "no")).await;
    let result = catalogue::download(
        &entry(),
        dir.path(),
        &missing.url,
        None,
        &CancellationToken::new(),
        |_| {},
    )
    .await;
    assert_eq!(result, Err(DownloadError::Status(404)));
}
