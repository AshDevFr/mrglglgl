//! An optional OpenAI-compatible endpoint: Ollama, LM Studio, llama-server or a cloud provider.
//! Never required; the bundled model is the default.

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{GenerateOutcome, GenerateRequest, Provider, ProviderError};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointConfig {
    /// The API's base address, the one ending in `/v1` for most servers.
    pub base_url: String,
    pub model: String,
    /// Sent as a bearer token when set. The app keeps it in the system keychain.
    #[serde(skip)]
    pub api_key: Option<String>,
}

/// Whether text sent to `base_url` leaves this machine. Anything that is not a loopback address
/// does, and the settings screen then says plainly what is sent.
pub fn sends_off_machine(base_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url) else {
        return true;
    };
    match url.host() {
        Some(url::Host::Domain(d)) => !d.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => !ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => !ip.is_loopback(),
        None => true,
    }
}

pub struct EndpointProvider {
    config: EndpointConfig,
    client: reqwest::Client,
    quirks: std::sync::Mutex<Quirks>,
}

/// What a server turned out to refuse, learned from its first 400 and kept for later requests.
/// OpenAI's newer models want `max_completion_tokens` in place of `max_tokens`, and some accept
/// no temperature but their own.
#[derive(Debug, Default, Clone, Copy)]
struct Quirks {
    completion_tokens: bool,
    no_temperature: bool,
}

impl Quirks {
    fn apply(self, body: &mut serde_json::Value) {
        let Some(map) = body.as_object_mut() else {
            return;
        };
        if self.completion_tokens
            && let Some(limit) = map.remove("max_tokens")
        {
            map.insert("max_completion_tokens".into(), limit);
        }
        if self.no_temperature {
            map.remove("temperature");
        }
    }

    /// The quirk a refusal names, if it is one not yet known.
    fn learn(&mut self, refusal: &str) -> bool {
        let refusal = refusal.to_lowercase();
        if !self.completion_tokens && refusal.contains("max_completion_tokens") {
            self.completion_tokens = true;
            return true;
        }
        if !self.no_temperature
            && refusal.contains("temperature")
            && (refusal.contains("unsupported") || refusal.contains("not support"))
        {
            self.no_temperature = true;
            return true;
        }
        false
    }
}

impl EndpointProvider {
    pub fn new(config: EndpointConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            quirks: std::sync::Mutex::default(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.config.base_url.trim_end_matches('/'))
    }

    fn host(&self) -> String {
        reqwest::Url::parse(&self.config.base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| self.config.base_url.clone())
    }

    fn authorised(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.api_key {
            Some(key) if !key.is_empty() => request.bearer_auth(key),
            _ => request,
        }
    }

    fn send_error(&self, e: &reqwest::Error) -> ProviderError {
        if e.is_connect() || e.is_timeout() || e.is_request() {
            ProviderError::Unreachable(self.host())
        } else {
            ProviderError::Failed(e.to_string())
        }
    }

    /// A web page where the API should answer: most often a sign-in page that a proxy in front
    /// of the server redirected to, which would otherwise read as no models and no text.
    fn web_page(&self, response: &reqwest::Response) -> Option<ProviderError> {
        let html = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|t| t.to_ascii_lowercase().starts_with("text/html"));
        html.then(|| {
            let landed = response.url().host_str().unwrap_or_default().to_string();
            ProviderError::Failed(if landed.is_empty() || landed == self.host() {
                format!("{} answered with a web page, not the API", self.host())
            } else {
                format!(
                    "{} sent the request on to a web page at {landed}, likely a sign-in page in \
                     front of the API: let the API through it, or use the server's own address",
                    self.host()
                )
            })
        })
    }

    fn quirks(&self) -> Quirks {
        self.quirks.lock().map(|q| *q).unwrap_or_default()
    }

    /// Posts a chat completion, adjusting to what the server refuses: a 400 naming a known quirk
    /// is retried once per quirk with the request changed to suit, and remembered.
    async fn post_chat(
        &self,
        mut body: serde_json::Value,
    ) -> Result<reqwest::Response, ProviderError> {
        loop {
            self.quirks().apply(&mut body);
            let response = self
                .authorised(self.client.post(self.url("chat/completions")))
                .json(&body)
                .send()
                .await
                .map_err(|e| self.send_error(&e))?;
            if let Some(e) = self.web_page(&response) {
                return Err(e);
            }
            if response.status() != StatusCode::BAD_REQUEST {
                return Ok(response);
            }
            let refusal = response.text().await.unwrap_or_default();
            let learned = self.quirks.lock().is_ok_and(|mut q| q.learn(&refusal));
            if !learned {
                return Err(self.error_for(StatusCode::BAD_REQUEST, &refusal));
            }
            tracing::info!("the endpoint wants the request adjusted, asking again: {refusal}");
        }
    }

    async fn status_error(&self, response: reqwest::Response) -> ProviderError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        self.error_for(status, &body)
    }

    fn error_for(&self, status: StatusCode, body: &str) -> ProviderError {
        let lower = body.to_lowercase();
        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::Unauthorised,
            StatusCode::NOT_FOUND => ProviderError::ModelNotFound(self.config.model.clone()),
            _ if lower.contains("model")
                && (lower.contains("not found") || lower.contains("does not exist")) =>
            {
                ProviderError::ModelNotFound(self.config.model.clone())
            }
            _ => ProviderError::Failed(format!(
                "the endpoint answered {}: {}",
                status.as_u16(),
                body.chars().take(200).collect::<String>()
            )),
        }
    }

    /// The models the endpoint lists at `models`, sorted. A server with no list there, or one
    /// that sends something other than a list, is an error that says so, so the player sees why
    /// there is nothing to pick from rather than an empty picker.
    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = self.url("models");
        tracing::debug!("listing models at {url}");
        let response = self
            .authorised(self.client.get(&url))
            .send()
            .await
            .map_err(|e| self.send_error(&e))?;
        if let Some(e) = self.web_page(&response) {
            return Err(e);
        }
        let status = response.status();
        match status {
            s if s.is_success() => {
                let body = response.text().await.unwrap_or_default();
                let ids = serde_json::from_str(&body)
                    .ok()
                    .and_then(|listed| model_ids(&listed))
                    .ok_or_else(|| {
                        ProviderError::Failed(format!(
                            "{url} did not answer with a list of models: {}",
                            body.chars().take(200).collect::<String>()
                        ))
                    })?;
                tracing::info!("{url} lists {} models", ids.len());
                Ok(ids)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::Unauthorised),
            _ => Err(ProviderError::Failed(format!(
                "{url} answered {}: no model list there",
                status.as_u16()
            ))),
        }
    }

    /// Checks the endpoint answers and accepts the key, and returns the models it lists. With a
    /// model chosen, also asks that model for a word: a list can be empty or wrong (a proxy may
    /// list models it cannot reach), and only an answer proves the model writes. A server that
    /// lists nothing can still write, so only an unreachable server or a refused key ends the
    /// test at the list, unless no model was named and the list is all there is to test.
    pub async fn test_connection(&self) -> Result<Vec<String>, ProviderError> {
        let model = self.config.model.trim();
        let models = match self.list_models().await {
            Ok(models) => models,
            // With no model to ask, the list is the whole test.
            Err(e) if model.is_empty() => return Err(e),
            Err(e @ (ProviderError::Unauthorised | ProviderError::Unreachable(_))) => {
                return Err(e);
            }
            Err(e) => {
                tracing::info!("no model list, testing {model} without one: {e}");
                Vec::new()
            }
        };
        if model.is_empty() {
            return Ok(models);
        }
        if !models.is_empty() && !models.iter().any(|m| m == model) {
            return Err(ProviderError::ModelNotFound(model.to_string()));
        }
        tracing::debug!("asking {model} for a word, to check it answers");
        let response = self
            .post_chat(json!({
                "model": model,
                "max_tokens": 8,
                "messages": [{"role": "user", "content": "Say hello."}],
            }))
            .await
            .map_err(|e| match e {
                ProviderError::Failed(message) if message.to_lowercase().contains("model") => {
                    tracing::debug!("the model check failed: {message}");
                    ProviderError::ModelNotFound(model.to_string())
                }
                other => other,
            })?;
        if !response.status().is_success() {
            return Err(match self.status_error(response).await {
                // Servers word an unknown model in many ways; the model was named, so say it.
                ProviderError::Failed(message) if message.to_lowercase().contains("model") => {
                    tracing::debug!("the model check failed: {message}");
                    ProviderError::ModelNotFound(model.to_string())
                }
                other => other,
            });
        }
        let answer: serde_json::Value = response.json().await.unwrap_or_default();
        if let Some(message) = error_message(&answer) {
            return Err(ProviderError::Failed(message));
        }
        Ok(models)
    }
}

/// The model names in a listing, sorted. OpenAI-compatible servers send `{"data": [{"id"}]}`;
/// some send a bare array, or `models` (Ollama's own shape), with `id`, `name` or `model` keys,
/// or plain strings.
fn model_ids(listed: &serde_json::Value) -> Option<Vec<String>> {
    let entries = listed
        .as_array()
        .or_else(|| listed["data"].as_array())
        .or_else(|| listed["models"].as_array())?;
    let mut ids: Vec<String> = entries
        .iter()
        .filter_map(|m| {
            m.as_str()
                .or_else(|| m["id"].as_str())
                .or_else(|| m["name"].as_str())
                .or_else(|| m["model"].as_str())
                .map(str::to_string)
        })
        .collect();
    ids.sort();
    ids.dedup();
    Some(ids)
}

/// The message of an error object, as OpenAI-compatible servers send it in a body or a stream.
fn error_message(value: &serde_json::Value) -> Option<String> {
    let error = value.get("error")?;
    Some(
        error["message"]
            .as_str()
            .or(error.as_str())
            .unwrap_or("the endpoint reported an error")
            .to_string(),
    )
}

/// One streamed chunk: its text, and whether it carried thinking instead (reasoning models
/// stream that apart, as `reasoning_content` or `reasoning`).
fn delta(line: &str) -> Result<(Option<String>, bool), ProviderError> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return Ok((None, false));
    };
    if let Some(message) = error_message(&value) {
        return Err(ProviderError::Failed(message));
    }
    let delta = &value["choices"][0]["delta"];
    let thinking = ["reasoning_content", "reasoning"]
        .iter()
        .any(|k| delta[k].as_str().is_some_and(|t| !t.is_empty()));
    Ok((delta["content"].as_str().map(str::to_string), thinking))
}

/// How a stream ended without any text, said plainly.
fn nothing_written(thought: bool) -> ProviderError {
    ProviderError::Failed(if thought {
        "the model spent its tokens thinking and wrote no text; turn its thinking off or choose \
         a model that does not think"
            .to_string()
    } else {
        "the endpoint answered but sent no text".to_string()
    })
}

#[async_trait]
impl Provider for EndpointProvider {
    fn id(&self) -> &str {
        "endpoint"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        cancel: CancellationToken,
        sink: mpsc::Sender<String>,
    ) -> Result<GenerateOutcome, ProviderError> {
        let body = json!({
            "model": self.config.model,
            "stream": true,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": request.user},
            ],
        });
        tracing::debug!(
            "asking {} for {} tokens of {}",
            self.url("chat/completions"),
            request.max_tokens,
            self.config.model
        );
        let response = tokio::select! {
            r = self.post_chat(body) => r?,
            () = cancel.cancelled() => return Ok(GenerateOutcome::Cancelled),
        };
        if !response.status().is_success() {
            let error = self.status_error(response).await;
            tracing::warn!("the endpoint refused: {error}");
            return Err(error);
        }
        tracing::debug!("the endpoint answered {}, streaming", response.status());

        let mut stream = response.bytes_stream();
        let mut pending: Vec<u8> = Vec::new();
        let (mut wrote, mut thought) = (false, false);
        // The end of the stream: complete with text, otherwise a failure, never an empty entry.
        let finished = |wrote: bool, thought: bool| {
            if wrote {
                Ok(GenerateOutcome::Completed)
            } else {
                Err(nothing_written(thought))
            }
        };
        loop {
            let chunk = tokio::select! {
                c = stream.next() => c,
                () = cancel.cancelled() => return Ok(GenerateOutcome::Cancelled),
            };
            let Some(chunk) = chunk else {
                return finished(wrote, thought);
            };
            pending.extend_from_slice(&chunk.map_err(|e| self.send_error(&e))?);
            // Server-sent events: one `data:` line per chunk, possibly split across reads.
            while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = pending.drain(..=end).collect();
                let line = String::from_utf8_lossy(&line);
                let Some(data) = line.trim().strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    return finished(wrote, thought);
                }
                let (text, thinking) = delta(data).inspect_err(|e| {
                    tracing::warn!("the endpoint reported an error while writing: {e}");
                })?;
                thought |= thinking;
                if let Some(text) = text.filter(|t| !t.is_empty()) {
                    wrote = true;
                    if sink.send(text).await.is_err() {
                        return Ok(GenerateOutcome::Cancelled);
                    }
                }
            }
        }
    }
}
