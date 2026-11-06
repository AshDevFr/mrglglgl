//! A scripted provider for tests and previews: no model, no network.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{GenerateOutcome, GenerateRequest, Provider, ProviderError};

/// How the fake behaves while "generating".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// Sends the whole reply at once.
    Instant,
    /// Sends one word at a time with this pause, checking cancellation between words.
    PerWord(Duration),
    /// Blocks a thread for this long without checking cancellation, like a model stuck in a
    /// long prompt evaluation.
    Stubborn(Duration),
}

/// Replies with scripted texts in order (the last one repeats), and remembers every request.
pub struct FakeProvider {
    replies: Mutex<VecDeque<Result<String, ProviderError>>>,
    last: Mutex<Option<Result<String, ProviderError>>>,
    pace: Pace,
    requests: Mutex<Vec<GenerateRequest>>,
}

impl FakeProvider {
    pub fn new(replies: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::scripted(replies.into_iter().map(|r| Ok(r.into())))
    }

    /// Replies that may also be errors.
    pub fn scripted(replies: impl IntoIterator<Item = Result<String, ProviderError>>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().collect()),
            last: Mutex::new(None),
            pace: Pace::Instant,
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn with_pace(mut self, pace: Pace) -> Self {
        self.pace = pace;
        self
    }

    /// Every request received so far.
    pub fn requests(&self) -> Vec<GenerateRequest> {
        self.requests.lock().unwrap().clone()
    }

    fn next_reply(&self) -> Result<String, ProviderError> {
        let mut replies = self.replies.lock().unwrap();
        let mut last = self.last.lock().unwrap();
        if let Some(reply) = replies.pop_front() {
            *last = Some(reply);
        }
        last.clone().unwrap_or_else(|| Ok(String::new()))
    }
}

#[async_trait]
impl Provider for FakeProvider {
    fn id(&self) -> &str {
        "fake"
    }

    fn model(&self) -> &str {
        "fake-model"
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        cancel: CancellationToken,
        sink: mpsc::Sender<String>,
    ) -> Result<GenerateOutcome, ProviderError> {
        self.requests.lock().unwrap().push(request);
        let reply = self.next_reply()?;
        match self.pace {
            Pace::Instant => {
                let _ = sink.send(reply).await;
            }
            Pace::PerWord(pause) => {
                for (i, word) in reply.split(' ').enumerate() {
                    if cancel.is_cancelled() {
                        return Ok(GenerateOutcome::Cancelled);
                    }
                    let chunk = if i == 0 {
                        word.to_string()
                    } else {
                        format!(" {word}")
                    };
                    let _ = sink.send(chunk).await;
                    tokio::time::sleep(pause).await;
                }
            }
            Pace::Stubborn(busy) => {
                tokio::task::spawn_blocking(move || std::thread::sleep(busy))
                    .await
                    .map_err(|e| ProviderError::Failed(e.to_string()))?;
                let _ = sink.send(reply).await;
            }
        }
        Ok(if cancel.is_cancelled() {
            GenerateOutcome::Cancelled
        } else {
            GenerateOutcome::Completed
        })
    }
}
