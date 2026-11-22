use super::{AcpModelSource, McpServer, project_mcp};
use crate::acp::options::ParsedAcpOptions;
use acp_types::{AcpSessionOptions, PermissionMode};
use agent_client_protocol::McpServerStdio;

#[test]
fn acp_platform_openclaw_contract_rejects_unenforceable_mcp_tool_lists() {
    let mut options = ParsedAcpOptions::<()> {
        full_access: true,
        mcp_servers: vec![McpServer::Stdio(McpServerStdio::new("fixture", "/fixture/mcp"))],
        ..Default::default()
    };
    let source = AcpModelSource::SessionModel;
    let mapped = project_mcp(&options, source).unwrap();
    assert_eq!(mapped["fixture"]["command"], "/fixture/mcp");

    for allowed_tools in [vec![], vec!["fixture__read".into()]] {
        options.session_options =
            Some(AcpSessionOptions { allowed_tools: Some(allowed_tools), ..Default::default() });
        let error = project_mcp(&options, source).unwrap_err();
        assert!(error.to_string().contains("session tool allowlist"));
    }

    options.session_options = None;
    options.full_access = false;
    options.permission_mode = Some(PermissionMode::ApproveReads);
    assert!(project_mcp(&options, source).is_err());
    assert!(project_mcp(&options, AcpModelSource::CliLocal).is_err());
    options.mcp_servers.clear();
    assert!(project_mcp(&options, source).unwrap().is_empty());
}
