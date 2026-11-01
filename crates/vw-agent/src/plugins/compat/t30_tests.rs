//! Real stdio MCP calls go through the session registry and normal executor.
use super::mcp_app_execution;
use crate::tools::{Tool, ToolRuntimeContext, ToolSpec};
use serde_json::json;
use std::sync::Arc;

#[test]
fn codex_plugin_compat_t30_session_binding_uses_real_mcp_and_revokes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("server.py"),
        include_bytes!("../../../testdata/plugin_compat/t30/server.py"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("resource.json"),
        include_bytes!("../../../testdata/plugin_compat/t29/resource.json"),
    )
    .unwrap();
    let server = agent_client_protocol::McpServer::Stdio(
        agent_client_protocol::McpServerStdio::new("weather", "python3")
            .args(vec![root.path().join("server.py").display().to_string()]),
    );
    let pool = super::mcp_connections::McpSessionConnections::default();
    let id = crate::tools::mcp_tool::mcp_tool_id("weather", "forecast").unwrap();
    let tool = crate::tools::mcp_tool::McpTool::from_session_server(
        server,
        "forecast",
        ToolSpec::new(&id, "forecast", json!({"type":"object"})).with_read_only(true),
        Arc::new(pool),
        Some(std::time::Duration::from_secs(5)),
    );
    let context =
        ToolRuntimeContext::new("t30-real-session", Some(root.path().display().to_string()))
            .with_session_tools(crate::session::runtime::SessionToolset::new(vec![
                Arc::new(tool) as Arc<dyn Tool>
            ]))
            .with_test_config(crate::config::Config::default());
    let result = crate::tools::execute_tool_call(&id, "{}", &context).unwrap();
    let handle = result.data["mcp_app"]["execution_binding"].as_str().expect("host binding");
    for (session, server, name) in [
        ("other", "weather", "forecast"),
        ("t30-real-session", "other", "forecast"),
        ("t30-real-session", "weather", "bash"),
    ] {
        assert!(mcp_app_execution::call(session, server, handle, name, json!({})).is_err());
    }
    assert!(
        crate::session::ui_store::with_tenant_scope_sync(
            vw_api_types::id::TenantScope::new("other-platform", "other-company").unwrap(),
            || mcp_app_execution::call(
                "t30-real-session",
                "weather",
                handle,
                "forecast",
                json!({})
            )
        )
        .is_err()
    );
    assert_eq!(std::fs::read_to_string(root.path().join("calls.log")).unwrap().lines().count(), 1);
    let result =
        mcp_app_execution::call("t30-real-session", "weather", handle, "forecast", json!({}))
            .unwrap();
    assert_eq!(result["structuredContent"]["temperature"], 12);
    let manager =
        Arc::new(crate::approval::ApprovalManager::from_config(&crate::config::AutonomyConfig {
            level: crate::security::AutonomyLevel::Supervised,
            ..Default::default()
        }));
    let approval_context = crate::tools::ToolUseContext::new(
        "t30-real-session",
        Some(root.path().display().to_string()),
    )
    .with_approval(manager.clone())
    .with_channel("desktop")
    .with_full_access_enabled(true)
    .with_bypass_non_cli_approval_for_turn(true);
    let context = context.clone().with_tool_use_context(approval_context);
    let mut approved_binding = crate::tools::ToolCallResult {
        data: json!({"mcp_app":{"server":"weather","tool":"forecast"}}),
        ..Default::default()
    };
    mcp_app_execution::register(&context, &id, &mut approved_binding).unwrap();
    let approval_handle =
        approved_binding.data["mcp_app"]["execution_binding"].as_str().unwrap().to_owned();
    for decision in [crate::approval::ApprovalResponse::No, crate::approval::ApprovalResponse::Yes]
    {
        let before_calls =
            std::fs::read_to_string(root.path().join("calls.log")).unwrap().lines().count();
        let handle = approval_handle.clone();
        let call = std::thread::spawn(move || {
            mcp_app_execution::call("t30-real-session", "weather", &handle, "forecast", json!({}))
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let request = loop {
            if let Some(request) =
                manager.list_non_cli_pending_requests(None, None, None).into_iter().next()
            {
                break request;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "app must request real approval despite turn bypass"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        manager.record_non_cli_pending_resolution(&request.request_id, decision.clone());
        let result = call.join().unwrap();
        if decision == crate::approval::ApprovalResponse::No {
            assert!(result.is_err());
            assert_eq!(
                std::fs::read_to_string(root.path().join("calls.log")).unwrap().lines().count(),
                before_calls
            );
        } else {
            assert_eq!(result.unwrap()["structuredContent"]["temperature"], 12);
        }
    }
    super::mcp_connection_sessions::close_session("t30-real-session", root.path());
    assert!(
        mcp_app_execution::call("t30-real-session", "weather", handle, "forecast", json!({}))
            .is_err()
    );
}
