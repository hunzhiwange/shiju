use vw_api_app_diagnostics::DiagnosticScopeQuery;
use vw_api_mcp_management::{McpRecordRef, McpCatalogRecord};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub gateway: String,
    pub tenant: vw_api_transport::TenantScope,
    pub project_path: Option<String>,
    pub credentials_generation: u64,
    pub query: DiagnosticScopeQuery,
    pub records: Vec<McpRecordRef>,
    pub connections: Vec<McpCatalogRecord>,
}
