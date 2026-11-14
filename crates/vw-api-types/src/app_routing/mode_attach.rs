//! Project-scoped mode transaction; native configuration and credentials stay executor-owned.
use crate::app_providers::mode_apply::ModeApplyAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingModeRequest {
    pub action: ModeApplyAction,
}

pub use crate::app_providers::mode_apply::ModeApplyResult as RoutingModeResult;
