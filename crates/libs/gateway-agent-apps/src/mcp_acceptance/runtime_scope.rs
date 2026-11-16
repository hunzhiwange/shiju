//! Runtime selection policy assertions backed by the real catalog.
use agent_app_services::mcp::McpCatalogStore;
use vw_api_transport::ProjectScope;
use vw_api_app_auth::AuthService;
use vw_api_mcp_management::{McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpCredentialRef, McpFailureCode, McpRuntimeSelection};
use super::catalog::http_draft;
pub fn prepare_and_assert(catalog: &McpCatalogStore, scope: &McpCatalogScope, tenant_catalog: &McpCatalogScope, project: &ProjectScope, other: &ProjectScope, url: &str) {
    let http = catalog.create(&scope, http_draft("remote", &url)).unwrap().record;
    let mut sse = http_draft("events", &url);
    sse.connection = McpConnectionDefinition::Sse { url: url.to_owned(), headers: Default::default() };
    let sse = catalog.create(&scope, sse).unwrap().record;
    assert_eq!(
        catalog.select_for_runtime(
            &other,
            &scope,
            &McpRuntimeSelection { project: other.clone(), record: http.clone() },
        ),
        Err(McpFailureCode::PermissionDenied),
    );
    for record in [http, sse] {
        let selected = McpRuntimeSelection { project: project.clone(), record };
        catalog.select_for_runtime(&project, &scope, &selected).unwrap();
        catalog.select_for_runtime(&project, &scope, &selected).unwrap();
    }

    let mut private = http_draft("private", &url);
    if let McpConnectionDefinition::StreamableHttp { headers, .. } = &mut private.connection {
        headers.insert(
            "Authorization".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::CodexOauth,
                    account_id: "fixture".into(),
                },
            },
        );
    }
    let private = catalog.create(&tenant_catalog, private).unwrap().record;
    assert_eq!(
        catalog.select_for_runtime(
            &project,
            &tenant_catalog,
            &McpRuntimeSelection { project: project.clone(), record: private },
        ),
        Err(McpFailureCode::PermissionDenied),
    );
    let same_name = catalog.create(&tenant_catalog, http_draft("remote", &url)).unwrap().record;
    assert_eq!(
        catalog.select_for_runtime(
            &project,
            &tenant_catalog,
            &McpRuntimeSelection { project: project.clone(), record: same_name },
        ),
        Err(McpFailureCode::Conflict),
    );

}
