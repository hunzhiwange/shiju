//! Real host loop integration; assertions and fixtures belong to agent-loop.
use super::run::run;
use crate::config::Config;
#[tokio::test]
async fn run_rejects_missing_message_before_starting_agent() {
    let error = run(Config::default(), None, None, None, 0.7).await.unwrap_err();

    assert!(error.to_string().contains("provide a message"));
}
#[tokio::test]
async fn run_propagates_default_agent_factory_error_for_unknown_provider() {
    let mut config = Config::default();
    config.memory.backend = "none".to_string();
    config.default_provider = Some("task-0034-unknown-provider".to_string());
    config.workspace_dir =
        std::env::temp_dir().join(format!("vibe-window-task-0034-{}", std::process::id()));

    let error = run(config, Some("hello".to_string()), None, None, 0.7)
        .await
        .expect_err("unknown provider should fail through the public run entrypoint");

    assert!(error.to_string().contains("Unknown provider: task-0034-unknown-provider"));
}
#[tokio::test]
async fn run_applies_overrides_and_returns_agent_response() {
    agent_loop::agent::test_support::run_tests::run_applies_overrides_and_returns_agent_response::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn run_uses_documented_provider_and_model_fallbacks() {
    agent_loop::agent::test_support::run_tests::run_uses_documented_provider_and_model_fallbacks::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn run_propagates_provider_error_without_recording_end_event() {
    agent_loop::agent::test_support::run_tests::run_propagates_provider_error_without_recording_end_event::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn run_accepts_empty_message_string() {
    agent_loop::agent::test_support::run_tests::run_accepts_empty_message_string::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
