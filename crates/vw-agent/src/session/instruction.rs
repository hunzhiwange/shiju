//! Session instruction loading is owned by session-engine.
pub use session_engine::instruction::*;
#[cfg(test)]
use session_instructions::fetch_url;
#[cfg(test)]
use crate::config;
#[cfg(test)]
#[path = "claude_protocol_parity_tests.rs"]
mod claude_protocol_parity_tests;
#[cfg(test)]
#[path = "../../../libs/session-engine/tests/host_suites/instruction_tests.rs"]
mod instruction_tests;
#[cfg(test)]
#[path = "instruction_rules.rs"]
mod instruction_rules;
