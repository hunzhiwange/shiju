//! Project MCP definitions for reviewed ACP mounts.
use agent_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::Response,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_mcp_management::{McpFailureCode, McpFailureResponse};

#[derive(Clone)]
pub struct PolicyFile(pub Option<PathBuf>);

pub fn router() -> Router {
    Router::new()
        .route("/project-mcp/definitions", post(super::project_mcp_create::create_catalog_record))
        .route(
            "/project-mcp/definitions/{id}",
            put(super::project_mcp_update::update_catalog_record),
        )
        .layer(Extension(PolicyFile(
            std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from),
        )))
        .layer(middleware::map_response(async |mut response: Response| {
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }))
}

pub type McpHttpError = (StatusCode, Json<McpFailureResponse>);

pub(super) fn failure(status: StatusCode, code: McpFailureCode) -> McpHttpError {
    (status, Json(McpFailureResponse { code }))
}

pub(super) fn access_error(status: StatusCode) -> McpHttpError {
    let code = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => McpFailureCode::PermissionDenied,
        StatusCode::BAD_REQUEST => McpFailureCode::InvalidDefinition,
        StatusCode::NOT_FOUND => McpFailureCode::NotFound,
        StatusCode::CONFLICT => McpFailureCode::Conflict,
        _ => McpFailureCode::TargetUnavailable,
    };
    failure(status, code)
}

pub(super) fn unavailable<T>(_: T) -> McpHttpError {
    failure(StatusCode::SERVICE_UNAVAILABLE, McpFailureCode::StorageUnavailable)
}

pub(super) fn open_database(
    database: Option<Extension<GlobalDatabase>>,
) -> Result<GlobalDatabase, McpHttpError> {
    match database {
        Some(Extension(database)) => Ok(database),
        None => GlobalDatabase::for_home(&agent_paths::paths().home).map_err(unavailable),
    }
}
