mod ask;
mod prompt;
mod sanitize;
mod wire;

pub use ask::{ask, prompt_for};

use crate::config::Config;
use anyhow::Result;
use std::time::Duration;
use ureq::http;
use wire::{ChatRequest, ChatResponse, Message, ModelsResponse};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no API key stored — run `cmdpal --setup`")]
    MissingKey,
    #[error("invalid API key — run `cmdpal --setup`")]
    Unauthorized,
    #[error("rate limited — retry in {0}s")]
    RateLimited(u64),
    #[error("{0}")]
    Endpoint(String),
    #[error("request failed: {0}")]
    Transport(String),
    #[error("unexpected response: {0}")]
    BadResponse(String),
    #[error("model returned no usable single-line command")]
    Unusable,
}

pub struct Client {
    agent: ureq::Agent,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl Client {
    pub fn new(
        base_url: &str,
        model: &str,
        timeout: Duration,
        temperature: f32,
        max_tokens: u32,
    ) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build();
        Self {
            agent: config.into(),
            base_url: base_url.to_string(),
            model: model.to_string(),
            temperature,
            max_tokens,
        }
    }

    pub fn from_config(config: &Config) -> Result<Self> {
        if config.ai.model.is_empty() {
            anyhow::bail!("no model configured — run `cmdpal --setup`");
        }
        Ok(Self::new(
            &config.ai.base_url,
            &config.ai.model,
            Duration::from_secs(config.ai.timeout_secs),
            config.ai.temperature,
            config.ai.max_tokens,
        ))
    }

    pub fn models(&self, key: &str) -> Result<Vec<String>, Error> {
        let call = self
            .agent
            .get(self.url("models"))
            .header("Authorization", format!("Bearer {key}"))
            .call();
        let body = self.body(self.send(call)?)?;
        let parsed: ModelsResponse =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse(e.to_string()))?;
        let mut ids: Vec<_> = parsed.data.into_iter().map(|model| model.id).collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub fn chat(&self, key: &str, messages: &[Message]) -> Result<String, Error> {
        let payload = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };
        let call = self
            .agent
            .post(self.url("chat/completions"))
            .header("Authorization", format!("Bearer {key}"))
            .header("Content-Type", "application/json")
            .send_json(payload);
        let body = self.body(self.send(call)?)?;
        let parsed: ChatResponse =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse(e.to_string()))?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| Error::BadResponse("no choices in response".into()))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn send(
        &self,
        call: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<ureq::http::Response<ureq::Body>, Error> {
        call.map_err(|error| Error::Transport(error.to_string()))
    }

    fn body(&self, response: ureq::http::Response<ureq::Body>) -> Result<String, Error> {
        let status = response.status();
        let retry_after = header(&response, "retry-after");
        let body = response
            .into_body()
            .read_to_string()
            .map_err(|error| Error::Transport(error.to_string()))?;
        if status.is_success() {
            return Ok(body);
        }
        Err(self.status_error(status, retry_after, &body))
    }

    fn status_error(
        &self,
        status: http::StatusCode,
        retry_after: Option<u64>,
        body: &str,
    ) -> Error {
        match status.as_u16() {
            401 | 403 => Error::Unauthorized,
            404 => Error::Endpoint(format!(
                "404 from {} — the base URL must be the full compatibility path, \
                 e.g. https://generativelanguage.googleapis.com/v1beta/openai/",
                self.base_url
            )),
            429 => Error::RateLimited(retry_after.unwrap_or(30)),
            503 | 529 => Error::Endpoint(format!("provider busy ({})", status.as_u16())),
            _ => Error::Endpoint(detail(status.as_u16(), body)),
        }
    }
}

fn header(response: &ureq::http::Response<ureq::Body>, name: &str) -> Option<u64> {
    response.headers().get(name)?.to_str().ok()?.parse().ok()
}

fn detail(status: u16, body: &str) -> String {
    let trimmed = body.trim();
    let message = serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(|message| message.as_str().map(String::from))
        })
        .unwrap_or_else(|| trimmed.to_string());
    if message.is_empty() {
        format!("HTTP {status}")
    } else {
        format!("HTTP {status}: {}", truncate(&message, 200))
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max).collect();
    format!("{head}…")
}
