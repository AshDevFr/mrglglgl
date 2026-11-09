//! The bundled local provider: llama.cpp through `llama-cpp-2`, on the CPU.
//!
//! One worker thread owns the model and runs one generation at a time, outside the async
//! runtime, so the window stays responsive. The model is loaded on the first request and dropped
//! after a quiet spell to give the memory back. Cancellation is checked between prompt chunks and
//! between generated tokens.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{GenerateOutcome, GenerateRequest, Provider, ProviderError};

/// The model is dropped after this long without a request.
pub const IDLE_UNLOAD: Duration = Duration::from_secs(120);
/// The largest context the provider opens, or the model's own if smaller. Each request opens
/// only what its prompt and answer need, so the memory of a long context is spent only on a
/// journal heavy enough to need it; ordinary prompts take a few thousand tokens.
const MAX_CONTEXT: u32 = 32_768;
/// Prompt tokens evaluated between cancellation checks.
const PROMPT_CHUNK: usize = 64;

/// How the provider runs a model.
#[derive(Debug, Clone)]
pub struct LocalOptions {
    pub model_path: PathBuf,
    /// The model's identifier, stored with each story (the catalogue id).
    pub model_id: String,
    /// The model has a reasoning mode, which is turned off: `/no_think` goes into the system
    /// message and an empty reasoning block is pre-filled, so it starts on the answer.
    pub reasoning: bool,
    /// CPU threads; `None` uses half the cores, leaving the rest for the window and the system.
    pub threads: Option<i32>,
    pub idle_unload: Duration,
}

impl LocalOptions {
    pub fn new(
        model_path: impl Into<PathBuf>,
        model_id: impl Into<String>,
        reasoning: bool,
    ) -> Self {
        Self {
            model_path: model_path.into(),
            model_id: model_id.into(),
            reasoning,
            threads: None,
            idle_unload: IDLE_UNLOAD,
        }
    }
}

struct Job {
    request: GenerateRequest,
    cancel: CancellationToken,
    sink: mpsc::Sender<String>,
    reply: oneshot::Sender<Result<GenerateOutcome, ProviderError>>,
}

pub struct LocalProvider {
    model_id: String,
    jobs: std_mpsc::Sender<Job>,
}

impl LocalProvider {
    /// Starts the worker. Nothing is loaded until the first request.
    pub fn new(options: LocalOptions) -> Self {
        let (jobs, inbox) = std_mpsc::channel::<Job>();
        let model_id = options.model_id.clone();
        std::thread::Builder::new()
            .name("mrglglgl-local-model".into())
            .spawn(move || worker(options, inbox))
            .expect("starting the local model thread");
        Self { model_id, jobs }
    }
}

#[async_trait]
impl Provider for LocalProvider {
    fn id(&self) -> &str {
        "local"
    }

    fn model(&self) -> &str {
        &self.model_id
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        cancel: CancellationToken,
        sink: mpsc::Sender<String>,
    ) -> Result<GenerateOutcome, ProviderError> {
        let (reply, answer) = oneshot::channel();
        self.jobs
            .send(Job {
                request,
                cancel,
                sink,
                reply,
            })
            .map_err(|_| ProviderError::Unavailable("the local model thread stopped".into()))?;
        answer
            .await
            .map_err(|_| ProviderError::Failed("the local model thread stopped".into()))?
    }
}

fn backend() -> Result<&'static LlamaBackend, ProviderError> {
    static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();
    BACKEND
        .get_or_init(|| {
            let mut backend = LlamaBackend::init().map_err(|e| e.to_string())?;
            backend.void_logs();
            Ok(backend)
        })
        .as_ref()
        .map_err(|e| ProviderError::Unavailable(format!("llama.cpp did not start: {e}")))
}

fn worker(options: LocalOptions, inbox: std_mpsc::Receiver<Job>) {
    let mut model: Option<LlamaModel> = None;
    loop {
        let job = match inbox.recv_timeout(options.idle_unload) {
            Ok(job) => job,
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                if model.take().is_some() {
                    tracing::info!("local model unloaded after {:?} idle", options.idle_unload);
                }
                continue;
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
        };
        let result = (|| {
            let backend = backend()?;
            if model.is_none() {
                let started = std::time::Instant::now();
                tracing::info!("loading the local model {}", options.model_path.display());
                model = Some(load(backend, &options.model_path).inspect_err(|e| {
                    tracing::warn!("the local model did not load: {e}");
                })?);
                tracing::info!("local model loaded in {:?}", started.elapsed());
            }
            let model = model.as_ref().expect("loaded above");
            tracing::debug!(
                "local model generating up to {} tokens",
                job.request.max_tokens
            );
            run(
                backend,
                model,
                &options,
                &job.request,
                &job.cancel,
                &job.sink,
            )
        })();
        let _ = job.reply.send(result);
    }
}

fn load(backend: &LlamaBackend, path: &Path) -> Result<LlamaModel, ProviderError> {
    if !path.is_file() {
        return Err(ProviderError::Unavailable(format!(
            "the model file is missing: {}",
            path.display()
        )));
    }
    // llama-cpp-2 builds Metal on Apple Silicon; offloading no layers keeps inference on the CPU,
    // which is what players' machines will do.
    let params = LlamaModelParams::default().with_n_gpu_layers(0);
    LlamaModel::load_from_file(backend, path, &params)
        .map_err(|e| ProviderError::Unavailable(format!("could not load the model: {e}")))
}

/// The most context a request may open: [`MAX_CONTEXT`], or less when the model was trained on
/// less. A model that does not say (0) gets the ceiling.
fn context_limit(trained: u32) -> u32 {
    if trained == 0 {
        MAX_CONTEXT
    } else {
        trained.min(MAX_CONTEXT)
    }
}

fn failed(e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Failed(e.to_string())
}

/// The prompt through the chat template stored in the GGUF. Templates without a system role
/// reject the system message, so then the instructions are folded into the user turn.
fn apply_template(model: &LlamaModel, system: &str, user: &str) -> Result<String, ProviderError> {
    let template = model
        .chat_template(None)
        .map_err(|e| failed(format!("the model has no chat template: {e}")))?;
    let message =
        |role: &str, text: &str| LlamaChatMessage::new(role.into(), text.into()).map_err(failed);
    let with_system = [message("system", system)?, message("user", user)?];
    match model.apply_chat_template(&template, &with_system, true) {
        Ok(prompt) => Ok(prompt),
        Err(_) => {
            let merged = [message("user", &format!("{system}\n\n{user}"))?];
            model
                .apply_chat_template(&template, &merged, true)
                .map_err(failed)
        }
    }
}

fn threads(options: &LocalOptions) -> i32 {
    options.threads.unwrap_or_else(|| {
        let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
        i32::try_from((cores / 2).max(1)).unwrap_or(1)
    })
}

fn run(
    backend: &LlamaBackend,
    model: &LlamaModel,
    options: &LocalOptions,
    request: &GenerateRequest,
    cancel: &CancellationToken,
    sink: &mpsc::Sender<String>,
) -> Result<GenerateOutcome, ProviderError> {
    let system = if options.reasoning {
        format!("{}\n/no_think", request.system)
    } else {
        request.system.clone()
    };
    let mut prompt = apply_template(model, &system, &request.user)?;
    if options.reasoning {
        prompt.push_str("<think>\n\n</think>\n\n");
    }
    // The chat template already writes any BOS text, so it is parsed rather than added again.
    let vocab = model.vocab();
    let tokens = vocab.tokenize(prompt.as_bytes(), false, true);
    if tokens.is_empty() {
        return Err(failed("the prompt is empty"));
    }
    let needed = tokens.len() as u32 + request.max_tokens + 16;
    let most = context_limit(model.n_ctx_train());
    if needed > most {
        return Err(failed(format!(
            "the prompt ({} tokens) is too long for the model's context ({most} tokens)",
            tokens.len()
        )));
    }
    tracing::debug!(
        "prompt of {} tokens, opening a context of {needed}",
        tokens.len()
    );
    let threads = threads(options);
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(needed))
        .with_n_batch(PROMPT_CHUNK as u32)
        .with_n_threads(threads)
        .with_n_threads_batch(threads);
    let mut ctx = model.new_context(backend, ctx_params).map_err(failed)?;

    let mut batch = LlamaBatch::new(PROMPT_CHUNK.max(1), 1);
    let last = tokens.len() - 1;
    for (chunk_index, chunk) in tokens.chunks(PROMPT_CHUNK).enumerate() {
        if cancel.is_cancelled() {
            return Ok(GenerateOutcome::Cancelled);
        }
        batch.clear();
        for (offset, token) in chunk.iter().enumerate() {
            let pos = chunk_index * PROMPT_CHUNK + offset;
            batch
                .add(*token, pos as i32, &[0], pos == last)
                .map_err(failed)?;
        }
        ctx.decode(&mut batch).map_err(failed)?;
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::top_k(40),
        LlamaSampler::top_p(0.95, 1),
        LlamaSampler::min_p(0.05, 1),
        LlamaSampler::temp(request.temperature),
        LlamaSampler::dist(seed),
    ]);

    let mut pending: Vec<u8> = Vec::new();
    let first = tokens.len() as i32;
    for position in (first..).take(request.max_tokens as usize) {
        if cancel.is_cancelled() {
            return Ok(GenerateOutcome::Cancelled);
        }
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if vocab.is_eog(token) {
            break;
        }
        let piece = vocab.token_to_piece(token, false, None);
        pending.extend_from_slice(&piece);
        // A character can be split across tokens; send only what is complete.
        let valid = match std::str::from_utf8(&pending) {
            Ok(s) => s.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(e) => e.valid_up_to() + e.error_len().unwrap_or(0),
        };
        if valid > 0 {
            let text = String::from_utf8_lossy(&pending[..valid]).into_owned();
            pending.drain(..valid);
            if sink.blocking_send(text).is_err() {
                // Nobody is listening any more: treat it as a cancel.
                return Ok(GenerateOutcome::Cancelled);
            }
        }
        batch.clear();
        batch.add(token, position, &[0], true).map_err(failed)?;
        ctx.decode(&mut batch).map_err(failed)?;
    }
    if !pending.is_empty() {
        let _ = sink.blocking_send(String::from_utf8_lossy(&pending).into_owned());
    }
    Ok(GenerateOutcome::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_context_is_the_ceiling_or_the_models_own_when_smaller() {
        assert_eq!(context_limit(262_144), MAX_CONTEXT);
        assert_eq!(context_limit(4_096), 4_096);
        assert_eq!(context_limit(0), MAX_CONTEXT);
    }
}
