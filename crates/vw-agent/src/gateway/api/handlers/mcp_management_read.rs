pub(crate) use gateway_mcp::server::mcp_management_read::{PolicyFile, McpHttpError};

pub(super) fn router() -> axum::Router {
    let routes = axum::Router::new();
    #[cfg(target_os = "macos")]
    let routes = routes
        .route("/agent-apps/mcp/checks", axum::routing::post(super::mcp_management_probe::submit)
            .layer(axum::extract::DefaultBodyLimit::max(32 * 1024)))
        .route("/agent-apps/mcp/tools/discover", axum::routing::post(super::mcp_management_tools::submit)
            .layer(axum::extract::DefaultBodyLimit::max(32 * 1024)))
        .route("/agent-apps/mcp/acp-mounts/query", axum::routing::get(super::mcp_management_tools::read_acp));
    gateway_mcp::server::mcp_management_read::router(routes)
}
