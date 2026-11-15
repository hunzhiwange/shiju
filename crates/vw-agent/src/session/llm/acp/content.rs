//! Bind the existing replay policy to ACP content construction.
use super::AcpReplayStrategy;
#[cfg(test)]
use serde_json::json;
pub(super) fn build_request_content(messages: &serde_json::Value, force_new_session: bool, strategy: AcpReplayStrategy, recent_count: usize) -> Result<acp_types::PromptInput, String> {
    llm_runtime::acp::content::build_request_content(messages, force_new_session, strategy, recent_count, super::replay::build_request_prompt)
}
#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
