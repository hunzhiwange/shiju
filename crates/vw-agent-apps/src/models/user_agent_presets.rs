//! Explicit User-Agent overrides for the model catalog.

pub use crate::providers::validation::custom_user_agent;

/// Preset values in display order. The selected string uses the same path as custom input.
pub const USER_AGENT_PRESETS: &[&str] = &[
    "claude-cli/2.1.161 (external, cli)",
    "claude-cli/2.1.161",
    "claude-code/1.0.0",
    "claude-code/0.1.0",
    "Kilo-Code/1.0",
];
