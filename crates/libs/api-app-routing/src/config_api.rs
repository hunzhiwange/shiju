//! Execution-host routing settings exposed by the control API.
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingConfig {
    /// Opt in to startup recovery on hosts whose administrator policy enables it.
    pub enabled: bool,
    pub listen_address: IpAddr,
    /// Zero asks the operating system to assign the port.
    pub listen_port: u16,
    pub logging_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppRoutingConfig {
    pub first_byte_timeout_secs: u16,
    /// Zero disables the streaming idle timeout.
    pub idle_timeout_secs: u16,
    pub non_streaming_timeout_secs: u16,
}
