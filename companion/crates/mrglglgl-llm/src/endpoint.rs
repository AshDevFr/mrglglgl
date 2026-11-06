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

    /// Checks the endpoint answers, accepts the key and has the model.
    pub async fn test_connection(&self) -> Result<(), ProviderError> {
        let response = self
            .authorised(self.client.get(self.url("models")))
            .send()
            .await
            .map_err(|e| self.send_error(&e))?;
        match response.status() {
            s if s.is_success() => {
                let listed: serde_json::Value = response.json().await.unwrap_or_default();
                let ids: Vec<&str> = listed["data"]
                    .as_array()
                    .map(|models| models.iter().filter_map(|m| m["id"].as_str()).collect())
                    .unwrap_or_default();
                // Some servers list nothing useful; only a list without the model is a no.
                if !ids.is_empty() && !ids.contains(&self.config.model.as_str()) {
                    return Err(ProviderError::ModelNotFound(self.config.model.clone()));
                }
                Ok(())
            }
            StatusCode::NOT_FOUND => Ok(()),
            _ => Err(self.status_error(response).await),
        }
    }
}

/// The text of one streamed chunk, from a `data:` line's JSON.
fn delta(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    value["choices"][0]["delta"]["content"]
        .as_str()
        .map(str::to_string)
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
        let send = self
            .authorised(self.client.post(self.url("chat/completions")))
            .json(&body)
            .send();
        let response = tokio::select! {
            r = send => r.map_err(|e| self.send_error(&e))?,
            () = cancel.cancelled() => return Ok(GenerateOutcome::Cancelled),
        };
        if !response.status().is_success() {
            return Err(self.status_error(response).await);
        }

        let mut stream = response.bytes_stream();
        let mut pending: Vec<u8> = Vec::new();
        loop {
            let chunk = tokio::select! {
                c = stream.next() => c,
                () = cancel.cancelled() => return Ok(GenerateOutcome::Cancelled),
            };
            let Some(chunk) = chunk else {
                return Ok(GenerateOutcome::Completed);
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
                    return Ok(GenerateOutcome::Completed);
                }
                if let Some(text) = delta(data).filter(|t| !t.is_empty())
                    && sink.send(text).await.is_err()
                {
                    return Ok(GenerateOutcome::Cancelled);
                }
            }
        }
    }
}
