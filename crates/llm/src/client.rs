use serde::{Deserialize, Serialize};
use serde_json::Value;
use world::ParsedAction;

use crate::analyst::{analyst_schema, parse_analyst_response};

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("ollama request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("ollama returned an empty response")]
    Empty,
}

pub struct OllamaClient {
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn is_up(&self) -> bool {
        (self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await)
            .is_ok_and(|r| r.status().is_success())
    }

    /// Free-text in-character dialogue. The prompt already encodes the referee verdict;
    /// the model only writes the performance.
    pub async fn perform(&self, prompt: &str) -> Result<String, LlmError> {
        let body = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
        };
        let resp: GenerateResponse = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let text = resp.response.trim().to_owned();
        if text.is_empty() {
            return Err(LlmError::Empty);
        }
        Ok(text)
    }

    /// Schema-constrained classification. Never errors outward: malformed output or a
    /// transport failure degrades to the conservative `inert` parse and is the caller's
    /// signal to fall back deterministically.
    pub async fn analyze(&self, system: &str, user: &str) -> ParsedAction {
        (self.request_analysis(system, user).await).map_or_else(
            |_| ParsedAction::inert(),
            |raw| parse_analyst_response(&raw),
        )
    }

    async fn request_analysis(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let body = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
            stream: false,
            format: analyst_schema(),
        };
        let resp: ChatResponse = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.message.content)
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    format: Value,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}
