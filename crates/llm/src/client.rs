use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use world::ParsedAction;

use crate::analyst::{
    analyst_schema, parse_reflection_response, reflect_schema, reflect_system_prompt,
    try_parse_analyst,
};

/// Each variant's message is a one-line, player-actionable summary. `NotRunning` and
/// `Auth` are kept distinct so the CLI can point at `ollama serve` vs `ollama signin`.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error(
        "can't reach Ollama — is it running? start it with `ollama serve` (or play with --offline)"
    )]
    NotRunning,
    #[error("Ollama rejected the request as unauthorized — sign in with `ollama signin`")]
    Auth,
    #[error("Ollama timed out without responding")]
    Timeout,
    #[error("Ollama returned HTTP {0}")]
    Status(u16),
    #[error("Ollama request failed: {0}")]
    Transport(String),
    #[error("Ollama returned an empty response")]
    Empty,
}

impl LlmError {
    fn classify(is_connect: bool, is_timeout: bool, status: Option<u16>, detail: &str) -> Self {
        if is_connect {
            LlmError::NotRunning
        } else if is_timeout {
            LlmError::Timeout
        } else {
            match status {
                Some(401 | 403) => LlmError::Auth,
                Some(code) => LlmError::Status(code),
                None => LlmError::Transport(detail.to_owned()),
            }
        }
    }
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        Self::classify(
            err.is_connect(),
            err.is_timeout(),
            err.status().map(|s| s.as_u16()),
            &err.to_string(),
        )
    }
}

/// Result of an Analyst call: the intent (inert if unparseable), the raw response for
/// inspection, and whether the model's output actually parsed.
pub struct Analysis {
    pub action: ParsedAction,
    pub raw: String,
    pub parsed: bool,
}

pub struct ReflectionResult {
    pub summary: Option<String>,
    pub raw: String,
}

pub struct OllamaClient {
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        // Short connect timeout so an unreachable host fails fast instead of hanging;
        // a generous overall timeout leaves room for slow local generation.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.into(),
            model: model.into(),
            http,
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Probe reachability. Classifies the failure so the caller can tell "not running"
    /// from "auth failed" rather than a bare bool.
    pub async fn health(&self) -> Result<(), LlmError> {
        self.http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
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

    /// Schema-constrained classification. Returns the parsed intent alongside the raw
    /// response and whether parsing succeeded, so the caller can surface a malformed
    /// (`format`-ignoring) model instead of silently riding the inert fallback. Only a
    /// transport/auth/HTTP failure is returned as an error.
    pub async fn analyze(&self, system: &str, user: &str) -> Result<Analysis, LlmError> {
        let raw = self
            .request_structured(system, user, analyst_schema())
            .await?;
        let action = try_parse_analyst(&raw);
        Ok(Analysis {
            parsed: action.is_some(),
            action: action.unwrap_or_else(ParsedAction::inert),
            raw,
        })
    }

    pub async fn reflect(&self, user: &str) -> Result<ReflectionResult, LlmError> {
        let raw = self
            .request_structured(reflect_system_prompt(), user, reflect_schema())
            .await?;
        Ok(ReflectionResult {
            summary: parse_reflection_response(&raw),
            raw,
        })
    }

    async fn request_structured(
        &self,
        system: &str,
        user: &str,
        format: Value,
    ) -> Result<String, LlmError> {
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
            format,
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

#[cfg(test)]
mod tests {
    use super::LlmError;

    #[test]
    fn connection_refused_is_not_running() {
        assert!(matches!(
            LlmError::classify(true, false, None, ""),
            LlmError::NotRunning
        ));
    }

    #[test]
    fn unauthorized_status_is_auth() {
        assert!(matches!(
            LlmError::classify(false, false, Some(401), ""),
            LlmError::Auth
        ));
        assert!(matches!(
            LlmError::classify(false, false, Some(403), ""),
            LlmError::Auth
        ));
    }

    #[test]
    fn other_status_is_reported_verbatim() {
        assert!(matches!(
            LlmError::classify(false, false, Some(500), ""),
            LlmError::Status(500)
        ));
    }

    #[test]
    fn timeout_is_distinct() {
        assert!(matches!(
            LlmError::classify(false, true, None, ""),
            LlmError::Timeout
        ));
    }

    #[test]
    fn auth_and_not_running_have_distinct_actionable_messages() {
        assert!(LlmError::Auth.to_string().contains("ollama signin"));
        assert!(LlmError::NotRunning.to_string().contains("ollama serve"));
    }
}
