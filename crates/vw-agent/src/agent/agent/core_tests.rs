#[test]
fn from_config_builds_agent_with_none_memory_and_supported_provider() {
    agent_loop::agent::test_support::core_tests::from_config_builds_agent_with_none_memory_and_supported_provider::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn from_config_rejects_unknown_provider() {
    agent_loop::agent::test_support::core_tests::from_config_rejects_unknown_provider::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn from_config_accepts_configured_custom_provider() {
    agent_loop::agent::test_support::core_tests::from_config_accepts_configured_custom_provider::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn from_config_rejects_invalid_programmatic_timezone() {
    agent_loop::agent::test_support::core_tests::from_config_rejects_invalid_programmatic_timezone::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[tokio::test]
async fn run_single_delegates_message_to_turn_and_returns_response() {
    agent_loop::agent::test_support::core_tests::run_single_delegates_message_to_turn_and_returns_response::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
