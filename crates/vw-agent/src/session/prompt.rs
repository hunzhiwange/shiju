//! Typed host reads and shared prompt composition.
pub use session_engine::prompt::{SystemPrompt, block_on, max_steps_text};
struct Host;
impl session_engine::prompt::PromptHost for Host {
    async fn environment_from_ref(model: Option<&str>, root: Option<&str>) -> String { super::system::environment_from_ref(model, root).await }
    async fn workspace_resolved(root: &std::path::Path) -> Vec<session_instructions::Resolved> { super::instruction::workspace_resolved(root).await }
    async fn system_resolved() -> Vec<session_instructions::Resolved> { super::instruction::system_resolved().await }
    fn recursive_policy() -> &'static str { crate::agent::prompt::multi_agent::recursive_orchestration_policy() }
}
pub fn system_with_metadata(model: Option<&str>, root: Option<&str>) -> SystemPrompt { session_engine::prompt::system_with_metadata::<Host>(model, root) }
pub fn system_with_agent_metadata(model: Option<&str>, root: Option<&str>, agent_key: Option<&str>, enabled_identity_files: &[String]) -> SystemPrompt { session_engine::prompt::system_with_agent_metadata::<Host>(model, root, agent_key, enabled_identity_files) }
pub fn system(model: Option<&str>, root: Option<&str>) -> String { session_engine::prompt::system::<Host>(model, root) }
#[cfg(test)]
fn environment(model: Option<&str>, root: Option<&str>) -> Vec<String> { vec![block_on(super::system::environment_from_ref(model, root))] }
#[cfg(test)]
#[path = "../../../libs/session-engine/tests/host_suites/prompt_tests.rs"]
mod prompt_tests;
