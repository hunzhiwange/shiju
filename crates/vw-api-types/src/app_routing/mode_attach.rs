//! Project-scoped mode transaction; native configuration and credentials stay executor-owned.
use vw_api_app_provider_environment::mode_apply::ModeApplyAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingModeRequest {
    pub action: ModeApplyAction,
}
pub use vw_api_app_provider_environment::mode_apply::ModeApplyResult as RoutingModeResult;
