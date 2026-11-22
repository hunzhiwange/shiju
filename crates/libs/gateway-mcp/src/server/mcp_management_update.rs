//! Update only the desired MCP definition using the caller's expected revision.

use axum::{
    Extension, Json,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use {
    vw_api_transport::ProjectScope,
    vw_api_transport::TenantScope,
    vw_api_mcp_management::McpCatalogRecord,
    vw_api_mcp_management::McpCatalogScope,
    vw_api_mcp_management::McpFailureCode,
    vw_api_mcp_management::McpRecordId,
    vw_api_mcp_management::McpRecordRef,
    vw_api_mcp_management::McpUpdateRequest,
};

use super::{
    agent_apps,
    mcp_management_read::{
        McpHttpError, PolicyFile, access_error, failure, open_database, unavailable,
    },
};
use {agent_app_services::mcp::McpCatalogStore, agent_database::GlobalDatabase};

pub(super) async fn update_catalog_record(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(id): Path<String>,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<McpUpdateRequest>, JsonRejection>,
) -> Result<Json<McpCatalogRecord>, McpHttpError> {
    let Json(request) = request.map_err(|error| {
        let status = match error.status() {
            StatusCode::PAYLOAD_TOO_LARGE => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::BAD_REQUEST,
        };
        failure(status, McpFailureCode::InvalidDefinition)
    })?;
    tokio::task::spawn_blocking(move || {
        let scope = match request.project_id.clone() {
            Some(project_id) => McpCatalogScope::Project {
                scope: ProjectScope::new(tenant, project_id)
                    .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
            },
            None => McpCatalogScope::Tenant { scope: tenant },
        };
        let expected = McpRecordRef { id: McpRecordId(id), revision: request.expected_revision };
        // Saving the desired definition makes no claim about native distribution or connectivity.
        let result = if let Some(action) = request.linux_stdio_selection {
            let McpCatalogScope::Project { scope: project } = &scope else {
                return Err(access_error(StatusCode::FORBIDDEN));
            };
            agent_apps::authorize_mcp_linux_stdio_review(
                policy.0.as_deref(),
                &headers,
                project,
                &request.target_host_id,
            )
            .map_err(access_error)?;
            if request.connection.is_some()
                || request.metadata != Default::default()
                || request.public_summary != Default::default()
            {
                return Err(failure(StatusCode::BAD_REQUEST, McpFailureCode::InvalidDefinition));
            }
            let store = McpCatalogStore::new(open_database(database)?).map_err(unavailable)?;
            store.configure_linux_stdio(project, &expected, action)
        } else {
            agent_apps::authorize_mcp_catalog_write(
                policy.0.as_deref(),
                &headers,
                &scope,
                &request.target_host_id,
            )
            .map_err(access_error)?;
            let store = McpCatalogStore::new(open_database(database)?).map_err(unavailable)?;
            store.update_record(&scope, &expected, request.connection, request.metadata, request.public_summary)
        };
        result.map(Json).map_err(|code| {
            let status = match code {
                McpFailureCode::InvalidDefinition => StatusCode::BAD_REQUEST,
                McpFailureCode::StaleRevision | McpFailureCode::Conflict => StatusCode::CONFLICT,
                McpFailureCode::NotFound => StatusCode::NOT_FOUND,
                McpFailureCode::PermissionDenied => StatusCode::FORBIDDEN,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            };
            failure(status, code)
        })
    })
    .await
    .map_err(unavailable)?
}

#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "mcp_management_review_tests.rs"]
mod review_tests;
