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
}

impl EndpointProvider {
    pub fn new(config: EndpointConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
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

    async fn status_error(&self, response: reqwest::Response) -> ProviderError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
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

    /// The models the endpoint lists, sorted; empty when it lists none (some servers do not).
    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        tracing::debug!("listing models at {}", self.url("models"));
        let response = self
            .authorised(self.client.get(self.url("models")))
            .send()
            .await
            .map_err(|e| self.send_error(&e))?;
        match response.status() {
            s if s.is_success() => {
                let listed: serde_json::Value = response.json().await.unwrap_or_default();
                let mut ids: Vec<String> = listed["data"]
                    .as_array()
                    .map(|models| {
                        models
                            .iter()
                            .filter_map(|m| m["id"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                ids.sort();
                ids.dedup();
                Ok(ids)
            }
            StatusCode::NOT_FOUND => Ok(Vec::new()),
            _ => Err(self.status_error(response).await),
        }
    }

    /// Checks the endpoint answers and accepts the key, and returns the models it lists. With a
    /// model chosen, also asks that model for a word: a list can be empty or wrong (a proxy may
    /// list models it cannot reach), and only an answer proves the model writes.
    pub async fn test_connection(&self) -> Result<Vec<String>, ProviderError> {
        let models = self.list_models().await?;
        let model = self.config.model.trim();
        if model.is_empty() {
            return Ok(models);
        }
        if !models.is_empty() && !models.iter().any(|m| m == model) {
            return Err(ProviderError::ModelNotFound(model.to_string()));
        }
        tracing::debug!("asking {model} for a word, to check it answers");
        let response = self
            .authorised(self.client.post(self.url("chat/completions")))
            .json(&json!({
                "model": model,
                "max_tokens": 8,
                "messages": [{"role": "user", "content": "Say hello."}],
            }))
            .send()
            .await
            .map_err(|e| self.send_error(&e))?;
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
        let send = self
            .authorised(self.client.post(self.url("chat/completions")))
            .json(&body)
            .send();
        let response = tokio::select! {
            r = send => r.map_err(|e| self.send_error(&e))?,
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
