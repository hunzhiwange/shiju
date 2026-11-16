//! Composition over the original routing listener and publication journal.
use axum::{Extension, Router};
use crate::global_database::GlobalDatabase;
use super::aggregation_codex_settings::SnapshotHost;
#[cfg(not(target_os = "macos"))]
impl gateway_aggregation::publication_host::Host for SnapshotHost { type Routing = (); }
pub(super) fn router() -> Router {
    let router = router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)), Err(_) => router,
    }
}
pub(super) fn router_with_policy(path: Option<std::path::PathBuf>) -> Router {
    gateway_aggregation::aggregation_members::router_with_policy(path, SnapshotHost)
}
