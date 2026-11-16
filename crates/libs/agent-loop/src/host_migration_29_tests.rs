//! Memory, history and credential behavior owned by the loop domain.
use crate::context::build_context;
use crate::core::constants::DEFAULT_MAX_HISTORY_MESSAGES;
use crate::core::history::{autosave_memory_key, build_native_assistant_history, build_native_assistant_history_from_parsed_calls};
use crate::history::{trim_history, apply_compaction_summary, build_compaction_transcript};
use crate::parsing::ParsedToolCall;
use crate::utils::scrub_credentials;
use model_types::ChatMessage;
#[path = "tests/loop_suite/memory_context.rs"]
mod memory_context;
