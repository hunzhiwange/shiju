//! Device synchronization routes with the real host projection adapter.
pub(crate) fn router() -> axum::Router {
    let router = gateway_device_sync::device_sync_jobs::router();
    #[cfg(target_os = "macos")]
    let router = router.layer(axum::middleware::from_fn(super::device_sync_projection::inject));
    router
}
