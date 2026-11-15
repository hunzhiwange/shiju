//! Host-level application routing control contracts.
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingStatus {
    pub state: RoutingState,
    pub bound_address: Option<IpAddr>,
    pub bound_port: Option<u16>,
    pub started_at: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingHealth {
    pub ready: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Map<String, serde_json::Value>")]
pub struct RoutingLifecycleRequest {}

impl TryFrom<serde_json::Map<String, serde_json::Value>> for RoutingLifecycleRequest {
    type Error = &'static str;

    fn try_from(fields: serde_json::Map<String, serde_json::Value>) -> Result<Self, Self::Error> {
        if !fields.is_empty() {
            return Err("expected an empty object");
        }
        Ok(Self {})
    }
}
