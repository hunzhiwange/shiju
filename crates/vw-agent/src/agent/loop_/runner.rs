//! Single-message host assembly.
pub async fn process_message(config: crate::config::Config, message: &str, session_id: &str) -> anyhow::Result<String> {
    agent_loop::runner::process_message::<super::core::skills::NativeCoreSkillHost>(config, message, session_id).await
}
#[cfg(test)]
#[path = "../../../../libs/agent-loop/src/runner_tests.rs"]
mod runner_tests;
