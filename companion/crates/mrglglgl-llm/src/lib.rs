//! Text generation providers. The bundled local model is the default and needs no external
//! tools or accounts; an OpenAI-compatible endpoint is optional.

use async_trait::async_trait;

/// What a provider is asked to write.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
}

/// How a generation ended when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerateOutcome {
    Completed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("the provider is not available: {0}")]
    Unavailable(String),
}

/// A source of generated text. Implementations stream text as it is produced.
#[async_trait]
pub trait Provider: Send + Sync {
    /// A stable identifier stored with every generated story.
    fn id(&self) -> &str;

    /// Generates text for `request`, passing each chunk to `on_chunk` as it arrives.
    async fn generate(
        &self,
        request: GenerateRequest,
        on_chunk: &mut (dyn FnMut(&str) + Send),
    ) -> Result<GenerateOutcome, ProviderError>;
}
