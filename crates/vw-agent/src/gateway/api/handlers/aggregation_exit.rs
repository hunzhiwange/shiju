//! Compose the exit HTTP route with the unique provider operation registry.
use super::aggregation_codex_settings::SnapshotHost;
#[cfg(not(target_os = "macos"))]
use gateway_aggregation::exit_host::ExitHost;
#[cfg(not(target_os = "macos"))]
impl ExitHost for SnapshotHost { type State = gateway_app_providers::operations::State; }
// ExitContext is implemented by the agent-gateway context owner.
pub(super) fn router() -> axum::Router { gateway_aggregation::aggregation_exit::router(SnapshotHost) }
