//! Text generation providers. The bundled local model is the default and needs no external
//! tools or accounts; an OpenAI-compatible endpoint is optional.
//!
//! Every provider streams text into a channel and stops when its cancellation token fires.
//! [`generate`] adds the per-request timeout on top and makes sure a provider that is slow to
//! notice cancellation still returns promptly.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
pub use tokio_util::sync::CancellationToken;

pub mod fake;
#[cfg(feature = "local")]
pub mod local;

/// Sampling temperature for journal prose: low enough to stay with the facts.
pub const DEFAULT_TEMPERATURE: f32 = 0.4;
/// Timeout for one generation on the bundled model.
pub const LOCAL_TIMEOUT: Duration = Duration::from_secs(300);
/// Timeout for one generation on an endpoint.
pub const ENDPOINT_TIMEOUT: Duration = Duration::from_secs(120);
/// How long a provider gets to notice cancellation before its result is abandoned.
pub const STOP_GRACE: Duration = Duration::from_millis(1500);

/// What a provider is asked to write.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateRequest {
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
    pub temperature: f32,
    /// The prose language, e.g. `"fr"`; informative, the prompt already asks for it.
    pub language: String,
}

/// How a generation ended when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerateOutcome {
    Completed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    #[error("no model is ready: {0}")]
    Unavailable(String),
    #[error("could not reach {0}")]
    Unreachable(String),
    #[error("the endpoint refused the key (unauthorised)")]
    Unauthorised,
    #[error("the endpoint has no model called {0}")]
    ModelNotFound(String),
    #[error("generation failed: {0}")]
    Failed(String),
}

/// A source of generated text.
#[async_trait]
pub trait Provider: Send + Sync {
    /// A stable identifier stored with every generated story, e.g. `"local"`.
    fn id(&self) -> &str;

    /// The model the text comes from, stored with every story.
    fn model(&self) -> &str;

    /// Generates text for `request`, sending chunks to `sink` as they are produced, until done
    /// or until `cancel` fires (then [`GenerateOutcome::Cancelled`]).
    async fn generate(
        &self,
        request: GenerateRequest,
        cancel: CancellationToken,
        sink: mpsc::Sender<String>,
    ) -> Result<GenerateOutcome, ProviderError>;
}

/// Runs `provider` with a timeout. Fires the provider's own cancellation when the timeout passes
/// or `cancel` fires, and gives it [`STOP_GRACE`] to stop before giving up on it, so either way
/// the caller has an answer within about two seconds.
pub async fn generate(
    provider: &dyn Provider,
    request: GenerateRequest,
    timeout: Duration,
    cancel: CancellationToken,
    sink: mpsc::Sender<String>,
) -> Result<GenerateOutcome, ProviderError> {
    let inner = cancel.child_token();
    let timed_out = Arc::new(AtomicBool::new(false));
    let timer = {
        let inner = inner.clone();
        let timed_out = timed_out.clone();
        tokio::spawn(async move {
            tokio::select! {
                () = tokio::time::sleep(timeout) => {
                    timed_out.store(true, Ordering::SeqCst);
                    inner.cancel();
                }
                () = inner.cancelled() => {}
            }
        })
    };
    let stopped = {
        let inner = inner.clone();
        async move {
            inner.cancelled().await;
            tokio::time::sleep(STOP_GRACE).await;
        }
    };
    let result = tokio::select! {
        r = provider.generate(request, inner.clone(), sink) => r,
        () = stopped => Ok(GenerateOutcome::Cancelled),
    };
    inner.cancel();
    timer.abort();
    match result {
        Ok(GenerateOutcome::Cancelled) if timed_out.load(Ordering::SeqCst) => {
            Ok(GenerateOutcome::TimedOut)
        }
        other => other,
    }
}

/// Like [`generate`], also collecting the text. Chunks are forwarded to `sink` when given.
pub async fn generate_text(
    provider: &dyn Provider,
    request: GenerateRequest,
    timeout: Duration,
    cancel: CancellationToken,
    sink: Option<mpsc::Sender<String>>,
) -> Result<(GenerateOutcome, String), ProviderError> {
    let (tx, mut rx) = mpsc::channel::<String>(64);
    let collect = async move {
        let mut text = String::new();
        while let Some(chunk) = rx.recv().await {
            text.push_str(&chunk);
            if let Some(sink) = &sink {
                // A closed sink only means nobody is watching; the text is still collected.
                let _ = sink.send(chunk).await;
            }
        }
        text
    };
    let (outcome, text) = tokio::join!(generate(provider, request, timeout, cancel, tx), collect);
    Ok((outcome?, text))
}
