//! Header-only checks for the application routing listener.

use axum::http::{Request, Uri, Version, header, uri::Authority};
use std::net::{IpAddr, SocketAddr};

pub(super) fn allows_host<B>(request: &Request<B>, local_addr: SocketAddr) -> bool {
    if !local_addr.ip().to_canonical().is_loopback() {
        return false;
    }
    // Absolute-form requests also carry an authority. A local Host header
    // cannot vouch for an unrelated authority in the request target.
    if request.uri().authority().is_some_and(|authority| !is_local(authority, local_addr)) {
        return false;
    }

    let mut hosts = request.headers().get_all(header::HOST).iter();
    match (hosts.next(), hosts.next()) {
        (Some(host), None) => host
            .to_str()
            .ok()
            .and_then(|host| host.parse::<Authority>().ok())
            .is_some_and(|authority| is_local(&authority, local_addr)),
        (None, None) => request.version() == Version::HTTP_10,
        _ => false,
    }
}

fn is_local(authority: &Authority, local_addr: SocketAddr) -> bool {
    let host = authority.host();
    if authority.as_str().contains('@')
        || (authority.as_str() != host && authority.port_u16() != Some(local_addr.port()))
    {
        return false;
    }
    // The current client endpoints use the bound loopback IP or localhost.
    // Never resolve a caller-supplied name to decide whether it is local.
    if host.strip_suffix('.').unwrap_or(host).eq_ignore_ascii_case("localhost") {
        return true;
    }
    let ip = host.strip_prefix('[').and_then(|host| host.strip_suffix(']')).unwrap_or(host);
    ip.parse::<IpAddr>().is_ok_and(|ip| ip.to_canonical() == local_addr.ip().to_canonical())
}

pub(super) fn is_desktop_route(uri: &Uri) -> bool {
    matches!(uri.path(), "/claude-desktop/v1/messages" | "/claude-desktop/v1/models")
}
