//! 使用真实执行、权限与 MCP 目录端口承接领域场景。
use super::services;
use agent_loop::host_migration_28_tests::tool_execution as scenarios;

#[test]
fn immediate_failure_preserves_tool_identity_and_marks_failure() {
    scenarios::immediate_failure_preserves_tool_identity_and_marks_failure(&services());
}

#[tokio::test]
async fn structured_progress_skips_when_result_dto_is_absent() {
    scenarios::structured_progress_skips_when_result_dto_is_absent(&services()).await;
}

#[tokio::test]
async fn excluded_tool_writes_blocked_result_without_execution() {
    scenarios::excluded_tool_writes_blocked_result_without_execution(&services()).await;
}

#[tokio::test]
async fn mcp_toolset_excluded_tool_is_blocked_by_channel_policy() {
    let tmp = tempfile::TempDir::new().unwrap();
    let workspace = scenarios::write_mcp_toolset_workspace(&tmp);
    let context = crate::tools::ToolRuntimeContext::new("mcp-toolset-test", Some(workspace.to_string_lossy().to_string()));
    let specs = crate::tools::tool_specs_for_context(&context);
    scenarios::mcp_toolset_excluded_tool_is_blocked_by_channel_policy(&services(), specs).await;
}

#[tokio::test]
async fn duplicate_text_tool_call_is_reported_without_second_execution() {
    scenarios::duplicate_text_tool_call_is_reported_without_second_execution(&services()).await;
}

#[tokio::test]
async fn native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages() {
    scenarios::native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages(&services()).await;
}

#[tokio::test]
async fn successful_tool_execution_emits_progress_and_structured_result() {
    scenarios::successful_tool_execution_emits_progress_and_structured_result(&services()).await;
}

#[tokio::test]
async fn read_only_tools_execute_through_parallel_batch_and_keep_history_order() {
    scenarios::read_only_tools_execute_through_parallel_batch_and_keep_history_order(&services()).await;
}

#[tokio::test]
async fn failed_tool_execution_emits_failure_progress_and_error_history() {
    scenarios::failed_tool_execution_emits_failure_progress_and_error_history(&services()).await;
}

#[tokio::test]
async fn cron_add_delivery_is_injected_before_tool_execution() {
    scenarios::cron_add_delivery_is_injected_before_tool_execution(&services()).await;
}

#[tokio::test]
async fn native_fallback_tool_ids_do_not_make_duplicate_calls_distinct() {
    scenarios::native_fallback_tool_ids_do_not_make_duplicate_calls_distinct(&services()).await;
}
