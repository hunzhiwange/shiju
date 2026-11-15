//! The facade keeps the domain DTO identities available at the original path.
use super::mcp_management_distribution::{McpBindingRequest, McpImportResponse};

#[test]
fn mcp_distribution_facade_preserves_domain_type_identity() {
    let response = McpImportResponse { sources: Vec::new() };
    let domain: gateway_mcp_distribution::McpImportResponse = response;
    assert!(domain.sources.is_empty());
    let request: Option<McpBindingRequest> = None;
    let domain: Option<gateway_mcp_distribution::McpBindingRequest> = request;
    assert!(domain.is_none());
}
