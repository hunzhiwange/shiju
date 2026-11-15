//! 以真实宿主能力运行所有者库的原测试断言。
#[test]
fn builder_returns_empty_builder_that_fails_without_required_parts() {
    agent_loop::agent::test_support::core_tests::builder_returns_empty_builder_that_fails_without_required_parts::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn history_returns_empty_slice_for_new_agent() {
    agent_loop::agent::test_support::core_tests::history_returns_empty_slice_for_new_agent::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn history_returns_messages_in_insertion_order() {
    agent_loop::agent::test_support::core_tests::history_returns_messages_in_insertion_order::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn clear_history_removes_all_messages() {
    agent_loop::agent::test_support::core_tests::clear_history_removes_all_messages::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn clear_history_is_idempotent_for_empty_history() {
    agent_loop::agent::test_support::core_tests::clear_history_is_idempotent_for_empty_history::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn trim_history_keeps_system_messages_and_recent_non_system_messages() {
    agent_loop::agent::test_support::core_tests::trim_history_keeps_system_messages_and_recent_non_system_messages::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn trim_history_keeps_all_non_system_messages_when_only_system_messages_exceed_limit() {
    agent_loop::agent::test_support::core_tests::trim_history_keeps_all_non_system_messages_when_only_system_messages_exceed_limit::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_system_prompt_includes_tool_protocol_for_prompt_guided_provider() {
    agent_loop::agent::test_support::core_tests::build_system_prompt_includes_tool_protocol_for_prompt_guided_provider::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[test]
fn build_system_prompt_omits_tool_protocol_for_native_provider() {
    agent_loop::agent::test_support::core_tests::build_system_prompt_omits_tool_protocol_for_native_provider::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
#[tokio::test]
async fn classify_model_preserves_explicit_model_over_classification_rules() {
    agent_loop::agent::test_support::core_tests::classify_model_preserves_explicit_model_over_classification_rules::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[tokio::test]
async fn classify_model_preserves_explicit_model_when_no_rule_matches() {
    agent_loop::agent::test_support::core_tests::classify_model_preserves_explicit_model_when_no_rule_matches::<crate::agent::loop_::core::skills::NativeCoreSkillHost>().await;
}
#[test]
fn legacy_and_processor_auto_selection_are_consistent() {
    agent_loop::agent::test_support::core_tests::legacy_and_processor_auto_selection_are_consistent::<crate::agent::loop_::core::skills::NativeCoreSkillHost>();
}
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
