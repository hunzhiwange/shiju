//! TLS setup and loop prevention for the application route transport.

use axum::http::StatusCode;
use rustls_platform_verifier::BuilderVerifierExt;
use std::sync::{Arc, OnceLock};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::{TlsConnector, client::TlsStream};

pub(super) fn tls_connector() -> Result<&'static TlsConnector, StatusCode> {
    static CONNECTOR: OnceLock<Option<TlsConnector>> = OnceLock::new();
    CONNECTOR
        .get_or_init(|| {
            let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .and_then(|builder| builder.with_platform_verifier())
            .ok()?
            .with_no_client_auth();
            config.alpn_protocols.push(b"http/1.1".to_vec());
            Some(TlsConnector::from(Arc::new(config)))
        })
        .as_ref()
        .ok_or(StatusCode::BAD_GATEWAY)
}

pub(super) async fn connect_tls<S: AsyncRead + AsyncWrite + Unpin>(
    connector: &TlsConnector,
    host: &str,
    socket: S,
) -> Result<TlsStream<S>, StatusCode> {
    let name = rustls_pki_types::ServerName::try_from(host.to_owned())
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    connector.connect(name, socket).await.map_err(|_| StatusCode::BAD_GATEWAY)
}

#[cfg(target_os = "macos")]
use axum::http::Uri;
#[cfg(target_os = "macos")]
use std::net::{IpAddr, SocketAddr};

/// Resolve names only when a request could reach the current listener's port.
#[cfg(target_os = "macos")]
pub(super) async fn check_origin(
    target: &reqwest::Url,
    listener: SocketAddr,
    proxied: bool,
) -> Result<Option<Vec<SocketAddr>>, StatusCode> {
    if target.port_or_known_default() != Some(listener.port()) {
        return Ok(None);
    }
    let domain = match target.host() {
        Some(url::Host::Ipv4(ip)) if !matches_listener(ip.into(), listener) => return Ok(None),
        Some(url::Host::Ipv6(ip)) if !matches_listener(ip.into(), listener) => return Ok(None),
        Some(url::Host::Domain(domain)) if !proxied => domain,
        // A proxy can resolve the name differently. Refuse this port unless
        // the transport can check and use the same addresses.
        _ => return Err(StatusCode::BAD_GATEWAY),
    };

    let addresses: Vec<_> = tokio::net::lookup_host((domain, listener.port()))
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .collect();
    if addresses.is_empty()
        || addresses.iter().any(|address| matches_listener(address.ip(), listener))
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    Ok(Some(addresses))
}

/// A known system proxy loop keeps the direct fallback. A same-port name
/// cannot be handed off to a proxy connector that would resolve it again.
#[cfg(target_os = "macos")]
pub(super) fn proxy_is_listener(uri: &Uri, listener: SocketAddr) -> Result<bool, StatusCode> {
    let port = uri.port_u16().or(match uri.scheme_str() {
        Some("http") => Some(80),
        Some("https") => Some(443),
        Some("socks4" | "socks4a" | "socks5" | "socks5h") => Some(1080),
        _ => None,
    });
    if port != Some(listener.port()) {
        return Ok(false);
    }
    let host = uri.host().ok_or(StatusCode::BAD_GATEWAY)?;
    if let Ok(ip) = host.trim_matches(['[', ']']).parse() {
        return Ok(matches_listener(ip, listener));
    }
    if super::outbound_system::is_loopback(host) {
        return Ok(true);
    }
    Err(StatusCode::BAD_GATEWAY)
}

#[cfg(target_os = "macos")]
fn matches_listener(target: IpAddr, listener: SocketAddr) -> bool {
    let ip = target.to_canonical();
    // Listener configuration only permits loopback bindings. Reject the
    // unspecified destination as macOS can route it to a loopback socket.
    ip.is_unspecified() || ip == listener.ip().to_canonical()
}
