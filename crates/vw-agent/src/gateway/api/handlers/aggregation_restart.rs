//! Compose the restart HTTP route with the unique provider operation registry.
use super::aggregation_codex_settings::SnapshotHost;
#[cfg(not(target_os = "macos"))]
use gateway_aggregation::restart_host::RestartHost;
// macOS registry binding belongs to gateway-app-routing.

#[cfg(not(target_os = "macos"))]
impl RestartHost for SnapshotHost { type State = gateway_app_providers::operations::State; }

pub(super) fn router() -> axum::Router { gateway_aggregation::aggregation_restart::router(SnapshotHost) }
