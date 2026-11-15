use crate::state::GatewayClientServerDraft;

pub fn server_health_key(server: &GatewayClientServerDraft) -> Option<String> {
    let host = server.host.trim();
    let host = if host.is_empty() { "127.0.0.1" } else { host };
    let raw = if host.contains("://") {
        host.to_string()
    } else {
        format!("http://{}:{}", host, server.port.clamp(1, u16::MAX))
    };
    let mut url = url::Url::parse(&raw).ok()?;
    if url.port().is_none() {
        let _ = url.set_port(Some(server.port.clamp(1, u16::MAX)));
    }
    let scheme = url.scheme();
    let host = url.host_str()?;
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    let port = url.port().unwrap_or_else(|| server.port.clamp(1, u16::MAX));
    Some(format!("{scheme}://{host}:{port}/v1/health"))
}
