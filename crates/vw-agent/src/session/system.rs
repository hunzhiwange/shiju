//! System prompt policy composition; rendering and environment logic live in session-engine.
use crate::provider::provider;
pub(crate) use session_engine::system_prompt::{RuntimePromptModes, UNIFIED_PROMPT_MARKER};
pub fn render_unified_prompt_with_modes(model_id: &str, working_directory: &str, available_tool_ids: &[String], runtime_modes: RuntimePromptModes) -> String { session_engine::system::render_unified_prompt_with_modes(model_id, working_directory, available_tool_ids, runtime_modes, crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub fn render_unified_prompt(model_id: &str, working_directory: &str, available_tool_ids: &[String]) -> String { session_engine::system::render_unified_prompt(model_id, working_directory, available_tool_ids, crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub fn render_unified_prompt_from_ref(model: Option<&str>, root: Option<&str>, available_tool_ids: &[String]) -> String { session_engine::system::render_unified_prompt_from_ref(model, root, available_tool_ids, crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub fn render_unified_prompt_for_model(model: &provider::Model, root: Option<&str>, available_tool_ids: &[String]) -> String { session_engine::system::render_unified_prompt_for_model(model, root, available_tool_ids, crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub fn instructions() -> String { session_engine::system::instructions(crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub fn provider_with_context(model: &provider::Model, root: Option<&str>, available_tool_ids: &[String]) -> Vec<String> { session_engine::system::provider_with_context(model, root, available_tool_ids, crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub fn provider(model: &provider::Model) -> Vec<String> { session_engine::system::provider(model, crate::agent::prompt::multi_agent::recursive_orchestration_policy()) }
pub async fn environment_from_ref(model: Option<&str>, root: Option<&str>) -> String {
    crate::provider::provider::ensure_host();
    session_engine::system::environment_from_ref(model, root).await
}
pub(crate) async fn team_runtime_instructions(session_id: &str, enabled: bool) -> String {
    session_engine::system::team_runtime_instructions::<super::session_io::NativeSessionIo>(session_id, enabled).await
}
#[cfg(test)]
use session_engine::system_prompt::resolve_cwd;
#[cfg(test)]
use session_engine::system::{environment_text, yes_no_en, platform, today_date_string, detect_git_from_dir, resolve_is_git};
#[cfg(test)]
const PROMPT_UNIFIED: &str = include_str!("../../../libs/session-engine/src/prompt/unified.txt");
#[cfg(test)]
#[path = "../../../libs/session-engine/tests/host_suites/system_tests.rs"]
mod system_tests;
