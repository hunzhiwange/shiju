//! Read-only VibeWindow session data rendered by copied status layouts.

use std::path::PathBuf;

mod card;
mod format;

pub use card::StatusHistoryCell;

/// Token counters already observed by the Gateway runtime.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct V3TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cached: u64,
    pub reasoning: u64,
}

impl V3TokenUsage {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.input.saturating_add(self.output)
    }
}

/// Display-only session snapshot supplied by the CLI bridge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3SessionDisplayData {
    pub version: String,
    pub cwd: PathBuf,
    pub gateway_endpoint: String,
    pub model: String,
    pub permissions: String,
    pub session_id: Option<String>,
    pub session_title: Option<String>,
    pub token_usage: V3TokenUsage,
    pub resumed: bool,
}
