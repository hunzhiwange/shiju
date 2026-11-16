//! 真实 AgentHost、动态目录和权限端口的领域场景接线。
use crate::agent::loop_::core::skills::NativeCoreSkillHost;
use agent_loop::host_migration_28_tests::discovery as scenarios;

#[tokio::test]
async fn codex_discovery_core_agent_discovers_and_calls_only_on_next_request() {
    scenarios::codex_discovery_core_agent_discovers_and_calls_only_on_next_request::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them() {
    scenarios::codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_same_round_search_does_not_allow_deferred_call() {
    scenarios::codex_discovery_core_same_round_search_does_not_allow_deferred_call::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent() {
    scenarios::codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_excluded_tool_cannot_be_searched_or_called() {
    scenarios::codex_discovery_core_excluded_tool_cannot_be_searched_or_called::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible() {
    scenarios::codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls() {
    scenarios::codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_empty_registry_provides_no_tool_protocol() {
    scenarios::codex_discovery_core_empty_registry_provides_no_tool_protocol::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks() {
    scenarios::codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn dynamic_tools_core_replacement_visible_next_request() {
    scenarios::dynamic_tools_core_replacement_visible_next_request::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn dynamic_tools_core_replacement_preserves_same_batch_rejection() {
    scenarios::dynamic_tools_core_replacement_preserves_same_batch_rejection::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn dynamic_tools_core_replacement_intersects_loaded_authorization() {
    scenarios::dynamic_tools_core_replacement_intersects_loaded_authorization::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn dynamic_tools_core_replacement_cannot_expand_child_authority() {
    scenarios::dynamic_tools_core_replacement_cannot_expand_child_authority::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn native_tool_search_core_loop_core_calls_discovered_tool_next_request() {
    scenarios::native_tool_search_core_loop_core_calls_discovered_tool_next_request::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn native_tool_search_core_loop_same_batch_remains_denied() {
    scenarios::native_tool_search_core_loop_same_batch_remains_denied::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools() {
    scenarios::native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn native_tool_search_core_loop_rejects_unadvertised_and_invalid_search() {
    scenarios::native_tool_search_core_loop_rejects_unadvertised_and_invalid_search::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn native_tool_search_core_loop_ordinary_discovery_still_executes() {
    scenarios::native_tool_search_core_loop_ordinary_discovery_still_executes::<NativeCoreSkillHost>().await;
}

#[tokio::test]
async fn native_tool_search_core_loop_replayed_search_is_not_completed_again() {
    scenarios::native_tool_search_core_loop_replayed_search_is_not_completed_again::<NativeCoreSkillHost>().await;
}
