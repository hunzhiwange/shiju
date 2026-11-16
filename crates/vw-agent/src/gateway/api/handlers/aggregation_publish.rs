//! Compose the publication HTTP route with the unique provider operation registry.
use super::aggregation_codex_settings::SnapshotHost;
#[cfg(not(target_os = "macos"))]
use gateway_aggregation::publish_host::PublishHost;
#[cfg(not(target_os = "macos"))]
impl PublishHost for SnapshotHost { type State = gateway_app_providers::operations::State; }
// PublishContext is implemented by the agent-gateway context owner.

pub(super) fn router() -> axum::Router { gateway_aggregation::aggregation_publish::router(SnapshotHost) }

pub(super) use gateway_aggregation::aggregation_publish::operation_error;
