mod analyst;
mod client;
mod config;
mod performer;

pub use analyst::{
    analyst_schema, analyst_system_prompt, heuristic_parse, parse_analyst_response,
    parse_reflection_response, reflect_schema, reflect_system_prompt, try_parse_analyst,
};
pub use client::{Analysis, LlmError, OllamaClient, ReflectionResult};
pub use config::{LlmConfig, DEFAULT_MODEL};
pub use performer::{build_performer_prompt, fallback_line, PerformerInput};
