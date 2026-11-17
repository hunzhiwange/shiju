//! ACP prompt replay implementation is owned by llm-runtime.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use llm_runtime::acp::replay::{build_replay_prompt, parse_replay_strategy};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use llm_runtime::acp::replay::{build_request_prompt, build_replay_prompt, parse_recent_count, parse_replay_strategy};
