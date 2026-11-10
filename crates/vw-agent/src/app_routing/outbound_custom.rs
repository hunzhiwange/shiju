//! Explicit outbound proxy construction and HTTP CONNECT for ordered requests.

use anyhow::{Result, anyhow, ensure};
use axum::http::{StatusCode, Uri};
use hyper_util::{
    client::{
        legacy::connect::proxy::Tunnel,
        proxy::matcher::{Intercept, Matcher},
    },
    rt::TokioIo,
};
use rustls_pki_types::ServerName;
use std::io;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};
use tokio_util::either::Either;
use tower::{ServiceExt, service_fn};

pub(super) fn parse(value: &str) -> Result<(reqwest::Proxy, Intercept)> {
    let mut url = reqwest::Url::parse(value).map_err(|_| anyhow!("invalid outbound proxy URL"))?;
    match url.scheme() {
        "http" | "https" => {}
        "socks5" | "socks5h" => {
            if url.port().is_none() {
                url.set_port(Some(1080)).map_err(|_| anyhow!("invalid outbound proxy port"))?;
            }
        }
        _ => anyhow::bail!("outbound proxy must use http, https, socks5 or socks5h"),
    }
    ensure!(
        url.has_host()
            && matches!(url.path(), "" | "/")
            && url.query().is_none()
            && url.fragment().is_none()
            && url.port() != Some(0),
        "invalid outbound proxy endpoint"
    );
    // Match both HTTP and HTTPS destinations. Explicit routing has no bypass
    // list and never inherits proxy settings from the environment.
    let intercept = Matcher::builder()
        .all(url.as_str())
        .build()
        .intercept(&Uri::from_static("https://localhost"))
        .ok_or_else(|| anyhow!("invalid outbound proxy endpoint"))?;
    let proxy = reqwest::Proxy::all(url).map_err(|_| anyhow!("invalid outbound proxy URL"))?;
    Ok((proxy, intercept))
}

pub(super) type ProxySocket = Either<TcpStream, TlsStream<TcpStream>>;

pub(super) async fn connect_tunnel(
    proxy: &Intercept,
    target: &Uri,
    tls: &TlsConnector,
) -> Result<ProxySocket, StatusCode> {
    let target_host = target.host().ok_or(StatusCode::BAD_GATEWAY)?;
    let port =
        target.port_u16().unwrap_or(if target.scheme_str() == Some("https") { 443 } else { 80 });
    let target = Uri::builder()
        .authority(format!("{target_host}:{port}"))
        .build()
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let tls = tls.clone();
    let mut tunnel =
        Tunnel::new(proxy.uri().clone(), service_fn(move |uri| connect_proxy(uri, tls.clone())));
    if let Some(auth) = proxy.basic_auth() {
        tunnel = tunnel.with_auth(auth.clone());
    }
    tunnel.oneshot(target).await.map(TokioIo::into_inner).map_err(|_| StatusCode::BAD_GATEWAY)
}

async fn connect_proxy(uri: Uri, tls: TlsConnector) -> io::Result<TokioIo<ProxySocket>> {
    let secure = uri.scheme_str() == Some("https");
    let host = uri
        .host()
        .ok_or_else(|| io::Error::other("proxy host missing"))?
        .trim_start_matches('[')
        .trim_end_matches(']');
    let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
    let tcp = TcpStream::connect((host, port)).await?;
    let socket = if secure {
        let name = ServerName::try_from(host.to_owned())
            .map_err(|_| io::Error::other("invalid proxy TLS server name"))?;
        Either::Right(tls.connect(name, tcp).await?)
    } else {
        Either::Left(tcp)
    };
    Ok(TokioIo::new(socket))
}
