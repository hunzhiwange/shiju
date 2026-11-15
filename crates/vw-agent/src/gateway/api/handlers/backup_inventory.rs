//! Backup inventory routing with the host authorization policy injected.
pub(super) fn router() -> axum::Router {
    gateway_backup_inventory::router(super::agent_apps::authorize_backup_inventory)
}
