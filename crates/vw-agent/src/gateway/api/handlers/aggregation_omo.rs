//! Host database composition for aggregation routes.
use axum::{Extension, Router};
use agent_database::GlobalDatabase;
pub(super) fn router() -> Router {
    let router = router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)), Err(_) => router,
    }
}
pub(super) use gateway_aggregation::aggregation_omo::router_with_policy;
