use std::path::Path;

use serde::Deserialize;

/// A small local instruct model. Overridable to any string — local names, or a
/// ":cloud" name the local Ollama server proxies through the same API.
pub const DEFAULT_MODEL: &str = "llama3.1";

/// Player-settable runtime config (a file now, a settings UI later). `model` is passed
/// to Ollama verbatim: no allowlist, no validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_owned(),
        }
    }
}

impl LlmConfig {
    /// A missing or malformed file yields defaults — config is a convenience, never a
    /// hard dependency.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_default_model() {
        assert_eq!(
            LlmConfig::load(Path::new("/no/such/file.toml")).model,
            DEFAULT_MODEL
        );
    }

    #[test]
    fn empty_config_keeps_default_model() {
        assert_eq!(
            toml::from_str::<LlmConfig>("").unwrap().model,
            DEFAULT_MODEL
        );
    }

    #[test]
    fn any_string_is_accepted_verbatim() {
        let cfg: LlmConfig = toml::from_str(r#"model = "gpt-oss:120b-cloud""#).unwrap();
        assert_eq!(cfg.model, "gpt-oss:120b-cloud");
    }
}
