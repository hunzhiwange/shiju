//! Host composition for scoped Codex settings; HTTP and native transaction logic live in the route crate.
use axum::{Extension, Router};
use crate::global_database::GlobalDatabase;
#[cfg(target_os = "macos")]
pub(super) use gateway_app_routing::server::snapshot_host::SnapshotHost;
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub(super) struct SnapshotHost;
#[cfg(not(target_os = "macos"))]
impl gateway_aggregation::aggregation_codex_settings::SnapshotHost for SnapshotHost {}
pub(super) fn router() -> Router {
    let router = router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)),
        Err(_) => router,
    }
}
pub(super) fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    gateway_aggregation::aggregation_codex_settings::router_with_policy(path, SnapshotHost)
}
