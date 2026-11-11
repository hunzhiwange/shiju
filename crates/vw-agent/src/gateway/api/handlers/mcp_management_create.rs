//! Create a definition in the authorized managed MCP catalog.

use axum::{
    Extension, Json,
    extract::rejection::JsonRejection,
    http::{HeaderMap, StatusCode},
};
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{McpCatalogRecord, McpCatalogScope, McpCreateRequest, McpFailureCode},
};

use super::{
    agent_apps,
    mcp_management_read::{
        McpHttpError, PolicyFile, access_error, failure, open_database, unavailable,
    },
};
use crate::{agent_apps::mcp::McpCatalogStore, global_database::GlobalDatabase};

pub(super) async fn create_catalog_record(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<McpCreateRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<McpCatalogRecord>), McpHttpError> {
    let Json(request) = request.map_err(|error| {
        let status = match error.status() {
            StatusCode::PAYLOAD_TOO_LARGE => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::BAD_REQUEST,
        };
        failure(status, McpFailureCode::InvalidDefinition)
    })?;
    tokio::task::spawn_blocking(move || {
        let McpCreateRequest {
            target_host_id,
            project_id,
            id,
            name,
            description,
            homepage,
            documentation,
            tags,
            connection,
        } = request;
        let scope = match project_id {
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
            &target_host_id,
        )
        .map_err(access_error)?;
        let store = McpCatalogStore::new(open_database(database)?).map_err(unavailable)?;
        let record = store
            .create(
                &scope,
                McpCatalogDraft {
                    id,
                    name,
                    description,
                    homepage,
                    documentation,
                    tags,
                    connection,
                },
            )
            .map_err(|code| {
                let status = match code {
                    McpFailureCode::InvalidDefinition => StatusCode::BAD_REQUEST,
                    McpFailureCode::Conflict => StatusCode::CONFLICT,
                    _ => StatusCode::SERVICE_UNAVAILABLE,
                };
                failure(status, code)
            })?;
        Ok((StatusCode::CREATED, Json(record)))
    })
    .await
    .map_err(unavailable)?
}
