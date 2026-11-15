//! Wire types for testing the execution host's configured proxy.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingOutboundProbeRequest {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingOutboundProbeEvidence {
    /// HTTP status from the fixed HTTPS target after establishing the proxy connection.
    pub http_status: u16,
}
