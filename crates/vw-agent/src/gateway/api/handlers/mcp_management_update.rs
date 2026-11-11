//! Update only the desired MCP definition using the caller's expected revision.

use axum::{
    Extension, Json,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{
        McpCatalogRecord, McpCatalogScope, McpFailureCode, McpRecordId, McpRecordRef,
        McpUpdateRequest,
    },
};

use super::{
    agent_apps,
    mcp_management_read::{
        McpHttpError, PolicyFile, access_error, failure, open_database, unavailable,
    },
};
use crate::{agent_apps::mcp::McpCatalogStore, global_database::GlobalDatabase};

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
        let scope = match request.project_id {
            Some(project_id) => McpCatalogScope::Project {
                scope: ProjectScope::new(tenant, project_id)
                    .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
            },
            None => McpCatalogScope::Tenant { scope: tenant },
        };
        agent_apps::authorize_mcp_catalog_write(
            policy.0.as_deref(),
            &headers,
            &scope,
            &request.target_host_id,
        )
        .map_err(access_error)?;
        let store = McpCatalogStore::new(open_database(database)?).map_err(unavailable)?;
        let expected = McpRecordRef { id: McpRecordId(id), revision: request.expected_revision };
        // Saving the desired definition makes no claim about native distribution or connectivity.
        store.update_connection(&scope, &expected, request.connection).map(Json).map_err(|code| {
            let status = match code {
                McpFailureCode::InvalidDefinition => StatusCode::BAD_REQUEST,
                McpFailureCode::StaleRevision | McpFailureCode::Conflict => StatusCode::CONFLICT,
                McpFailureCode::NotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            };
            failure(status, code)
        })
    })
    .await
    .map_err(unavailable)?
}
