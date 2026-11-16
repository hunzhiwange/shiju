#[test]
fn builder_returns_empty_builder_that_fails_without_required_parts() {
    agent_loop::agent::test_support::core_tests::builder_returns_empty_builder_that_fails_without_required_parts::<super::host::DomainHost>();
}
#[test]
fn history_returns_empty_slice_for_new_agent() {
    agent_loop::agent::test_support::core_tests::history_returns_empty_slice_for_new_agent::<super::host::DomainHost>();
}
#[test]
fn history_returns_messages_in_insertion_order() {
    agent_loop::agent::test_support::core_tests::history_returns_messages_in_insertion_order::<super::host::DomainHost>();
}
#[test]
fn clear_history_removes_all_messages() {
    agent_loop::agent::test_support::core_tests::clear_history_removes_all_messages::<super::host::DomainHost>();
}
#[test]
fn clear_history_is_idempotent_for_empty_history() {
    agent_loop::agent::test_support::core_tests::clear_history_is_idempotent_for_empty_history::<super::host::DomainHost>();
}
#[test]
fn trim_history_keeps_system_messages_and_recent_non_system_messages() {
    agent_loop::agent::test_support::core_tests::trim_history_keeps_system_messages_and_recent_non_system_messages::<super::host::DomainHost>();
}
#[test]
fn trim_history_keeps_all_non_system_messages_when_only_system_messages_exceed_limit() {
    agent_loop::agent::test_support::core_tests::trim_history_keeps_all_non_system_messages_when_only_system_messages_exceed_limit::<super::host::DomainHost>();
}
#[test]
fn build_system_prompt_includes_tool_protocol_for_prompt_guided_provider() {
    agent_loop::agent::test_support::core_tests::build_system_prompt_includes_tool_protocol_for_prompt_guided_provider::<super::host::DomainHost>();
}
#[test]
fn build_system_prompt_omits_tool_protocol_for_native_provider() {
    agent_loop::agent::test_support::core_tests::build_system_prompt_omits_tool_protocol_for_native_provider::<super::host::DomainHost>();
}
#[tokio::test]
async fn classify_model_preserves_explicit_model_over_classification_rules() {
    agent_loop::agent::test_support::core_tests::classify_model_preserves_explicit_model_over_classification_rules::<super::host::DomainHost>().await;
}
#[tokio::test]
async fn classify_model_preserves_explicit_model_when_no_rule_matches() {
    agent_loop::agent::test_support::core_tests::classify_model_preserves_explicit_model_when_no_rule_matches::<super::host::DomainHost>().await;
}
#[test]
fn legacy_and_processor_auto_selection_are_consistent() {
    agent_loop::agent::test_support::core_tests::legacy_and_processor_auto_selection_are_consistent::<super::host::DomainHost>();
}
