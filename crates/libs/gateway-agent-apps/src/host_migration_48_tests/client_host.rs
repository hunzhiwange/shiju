pub(super) struct FixtureHost;
impl mcp_client::ClientHost for FixtureHost {
    fn runtime_stdio_cwd(&self, server: &agent_client_protocol::McpServerStdio) -> anyhow::Result<Option<std::path::PathBuf>> {
        agent_app_services::mcp::runtime::managed_stdio_cwd(server)
    }
    fn http_headers<'a>(&'a self, _: &'a agent_client_protocol::McpServerHttp, _: Option<&'a mcp_client::FormOwner>)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Option<reqwest_012::header::HeaderMap>>> + Send + 'a>> {
        Box::pin(async { Ok(None) })
    }
    fn request_elicitation(&self, _: mcp_client::FormOwner, _: String,
        _: rmcp::model::CreateElicitationRequestParam, _: tokio_util::sync::CancellationToken,
        _: tokio_util::sync::CancellationToken, _: Option<serde_json::Value>)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<rmcp::model::CreateElicitationResult, rmcp::model::ErrorData>> + Send + '_>> {
        Box::pin(async { Err(rmcp::model::ErrorData::internal_error("fixture has no interaction owner", None)) })
    }
}
