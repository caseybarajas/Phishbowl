mod analyst;
mod client;
mod performer;

pub use analyst::{analyst_schema, analyst_system_prompt, heuristic_parse, parse_analyst_response};
pub use client::{LlmError, OllamaClient};
pub use performer::{build_performer_prompt, fallback_line, PerformerInput};
