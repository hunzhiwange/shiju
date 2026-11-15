//! Fixed execution-host loopback scan. Open TCP ports are protocol candidates only.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingOutboundScanRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingOutboundScanEvidence {
    pub port: u16,
    pub tcp_open: bool,
    /// Source protocol hints, not a successful protocol handshake.
    pub protocol_candidates: Vec<String>,
}
