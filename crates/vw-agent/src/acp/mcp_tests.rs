//! 测试 ACP 会话中 MCP 服务的作用域、传输转换与原子失败边界。
//! 临时 MCP 进程用于确认环境变量不泄漏，且部分准备失败不会留下半初始化工具。

use tempfile::TempDir;

use super::mcp::prepare_session_tools;
use crate::app::agent::tools::{ToolRuntimeContext, tool_specs_for_context};

use agent_acp::host_migration_53_mcp_contracts::fixture_server;

#[tokio::test]
async fn mcp_servers_are_scoped_to_bound_session() {
    let root = TempDir::new().expect("temporary root");
    let tools = prepare_session_tools(&[fixture_server(root.path(), Vec::new())], None)
        .await
        .expect("session tools");
    let with_mcp =
        ToolRuntimeContext::new("with-mcp", Some(root.path().to_string_lossy().into_owned()))
            .with_session_tools(tools);
    let without_mcp =
        ToolRuntimeContext::new("without-mcp", Some(root.path().to_string_lossy().into_owned()));

    assert!(tool_specs_for_context(&with_mcp).iter().any(|spec| spec.id == "mcp__fixture__echo"));
    assert!(
        !tool_specs_for_context(&without_mcp).iter().any(|spec| spec.id == "mcp__fixture__echo")
    );
}

#[tokio::test]
async fn mcp_conversion_preserves_supported_transport_without_logging_env() {
    let root = TempDir::new().expect("temporary root");
    agent_acp::host_migration_53_mcp_contracts::mcp_conversion_preserves_supported_transport_without_logging_env(
        root.path(), crate::plugins::compat::mcp_connections::host(),
    ).await;
}

#[tokio::test]
async fn unsupported_or_failed_mcp_is_rejected_atomically() {
    let root = TempDir::new().expect("temporary root");
    agent_acp::host_migration_53_mcp_contracts::unsupported_or_failed_mcp_is_rejected_atomically(
        root.path(), crate::plugins::compat::mcp_connections::host(),
    ).await;
}

#[tokio::test]
async fn loaded_session_rebuilds_requested_mcp_scope() {
    agent_acp::host_migration_53_mcp_contracts::loaded_session_rebuilds_requested_mcp_scope::<super::backend::NativeAcpHost>().await;
}
#[tokio::test]
async fn foreign_session_load_rejects_before_starting_mcp_processes() {
    agent_acp::host_migration_53_mcp_contracts::foreign_session_load_rejects_before_starting_mcp_processes::<super::backend::NativeAcpHost>().await;
}
