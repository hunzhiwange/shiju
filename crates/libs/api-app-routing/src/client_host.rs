//! Client connection hosts are distinct from listener bind addresses.
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingClientHostUpdate {
    pub host: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingClientHost {
    pub host: Option<String>,
    pub effective_host: String,
}

/// Normalize a host, never accepting a URL, port, zone id or wildcard address.
pub fn normalize_routing_client_host(host: Option<&str>) -> Result<Option<String>, &'static str> {
    let host = host.unwrap_or_default().trim();
    if host.is_empty() {
        return Ok(None);
    }
    let ip_host = if host.starts_with('[') && host.ends_with(']') {
        &host[1..host.len() - 1]
    } else {
        host
    };
    if let Ok(ip) = ip_host.parse::<IpAddr>() {
        let ip = ip.to_canonical();
        if ip.is_unspecified() || ip.is_multicast() {
            return Err("Client host must be a reachable unicast address");
        }
        return Ok(Some(ip.to_string()));
    }
    if host.len() > 253 || !host.is_ascii() || host.parse::<u32>().is_ok() {
        return Err("Invalid client hostname");
    }
    let name = host.strip_suffix('.').unwrap_or(host);
    if !name.split('.').all(|label| {
        !label.is_empty() && label.len() <= 63
            && label.as_bytes()[0].is_ascii_alphanumeric()
            && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            && label.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    }) {
        return Err("Expected an IP address or hostname without a port");
    }
    // Reject invalid dotted IPv4 instead of treating it as DNS.
    if name.bytes().all(|c| c.is_ascii_digit() || c == b'.') {
        return Err("Invalid IP address");
    }
    Ok(Some(host.to_ascii_lowercase()))
}

pub fn routing_client_host_from_bind(bind: IpAddr) -> String {
    let bind = bind.to_canonical();
    if bind.is_unspecified() {
        if bind.is_ipv4() { "127.0.0.1".into() } else { "::1".into() }
    } else {
        bind.to_string()
    }
}
