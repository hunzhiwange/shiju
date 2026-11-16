//! Registry and input sanitization adapters for session-engine policies.
use serde_json::Value;
use std::collections::HashSet;
use super::runner_host::AgentRunnerHost as AgentHelperHost;
pub(crate) use session_engine::helpers::{allowed_tool_ids_for_request_from_specs, response_preview, is_acp_request, should_execute_structured_tool_calls_locally};
pub(crate) fn allowed_tool_ids(model: Option<&str>) -> HashSet<String> { session_engine::helpers::allowed_tool_ids::<AgentHelperHost>(model) }
pub(crate) fn allowed_tool_ids_for_request(model: Option<&str>, options: &Value) -> HashSet<String> { session_engine::helpers::allowed_tool_ids_for_request::<AgentHelperHost>(model, options) }
pub(crate) fn tool_call_preview(tool_name: &str, arguments: &str) -> String { session_engine::helpers::tool_call_preview::<AgentHelperHost>(tool_name, arguments) }
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/helpers_tests.rs"]
mod helpers_tests;

pub(crate) fn query_has_any_tool_calls(query: &str) -> bool { session_engine::tool_parsing::query_has_any_tool_calls::<AgentHelperHost>(query) }
