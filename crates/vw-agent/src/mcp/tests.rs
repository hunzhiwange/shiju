use super::*;
use crate::tools::ToolRuntimeContext;
use rmcp::handler::server::ServerHandler;

#[test]
fn server_can_be_constructed_with_runtime_context() {
    let _server = AgentToolServer::new(ToolRuntimeContext::for_specs());
}

#[test]
fn server_info_exposes_stable_identity() {
    let server = AgentToolServer::new(ToolRuntimeContext::for_specs());
    let info = ServerHandler::get_info(&server);

    assert_eq!(info.server_info.name, "vibe-window");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(info.instructions.as_deref(), Some("Expose vibe-window agent tools over MCP."));
}

#[test]
fn unknown_tool_call_is_rejected_without_execution() {
    let ctx = NativeRuntime(ToolRuntimeContext::for_specs());

    let result = McpToolRuntime::execute_tool_call(&ctx, "definitely_missing_tool", "{}");

    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err()).contains("Unknown tool: definitely_missing_tool"));
}

#[test]
fn registered_tool_call_with_missing_arguments_is_rejected_before_execution() {
    let ctx = NativeRuntime(ToolRuntimeContext::for_specs());

    let result = McpToolRuntime::execute_tool_call(&ctx, "file_read", "{}");

    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err()).contains("path"));
}
