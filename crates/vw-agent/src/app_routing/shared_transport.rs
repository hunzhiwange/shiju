#[cfg(target_os = "macos")]
use super::outbound_system::{self, SystemProxy};
use super::{header_wire::HeaderWire, outbound_custom};
use crate::model_relay::upstream;
use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode, Uri, header},
    response::Response,
};
use futures_util::StreamExt;
use hyper::body::Incoming;
#[cfg(not(target_os = "macos"))]
use hyper_util::client::proxy::matcher::Matcher;
use hyper_util::{client::proxy::matcher::Intercept, rt::TokioIo};
use rustls_platform_verifier::BuilderVerifierExt;
use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, join, sink, split},
    net::TcpStream,
    task::JoinSet,
    time::{Instant, timeout_at},
};
use tokio_rustls::TlsConnector;
use tokio_util::{either::Either, sync::CancellationToken};

enum Egress {
    #[cfg(not(target_os = "macos"))]
    System(Matcher),
    #[cfg(target_os = "macos")]
    System(Arc<SystemProxy>),
    Custom(Intercept),
}

/// HTTP transport for the application router, independent of session relay bindings.
pub struct SharedTransport {
    client: reqwest::Client,
    encoded_client: reqwest::Client,
    egress: Egress,
    #[cfg(target_os = "macos")]
    direct: DirectClients,
}

#[cfg(target_os = "macos")]
struct DirectClients {
    client: reqwest::Client,
    encoded: reqwest::Client,
}

#[cfg(target_os = "macos")]
impl DirectClients {
    fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: upstream::upstream_http_client_builder().no_proxy().build()?,
            encoded: encoded_client_builder().no_proxy().build()?,
        })
    }
}

impl SharedTransport {
    /// Build an explicit egress for publication with an authorized route.
    pub fn with_custom_proxy(proxy_url: &str) -> anyhow::Result<Self> {
        let (proxy, intercept) = outbound_custom::parse(proxy_url)?;
        let build_error = |_| anyhow::anyhow!("could not build outbound proxy transport");
        Ok(Self {
            client: upstream::upstream_http_client_builder()
                .retry(reqwest::retry::never())
                .no_proxy()
                .proxy(proxy.clone())
                .build()
                .map_err(build_error)?,
            encoded_client: encoded_client_builder()
                .no_proxy()
                .proxy(proxy)
                .build()
                .map_err(build_error)?,
            egress: Egress::Custom(intercept),
            #[cfg(target_os = "macos")]
            direct: DirectClients::new().map_err(build_error)?,
        })
    }

    pub fn new() -> Result<Self, reqwest::Error> {
        #[cfg(target_os = "macos")]
        {
            Self::with_system_proxy(SystemProxy::read())
        }

        #[cfg(not(target_os = "macos"))]
        Ok(Self {
            client: upstream::upstream_http_client()?,
            encoded_client: encoded_client_builder().build()?,
            egress: Egress::System(Matcher::from_system()),
        })
    }

    #[cfg(target_os = "macos")]
    pub(super) fn with_system_proxy(system: SystemProxy) -> Result<Self, reqwest::Error> {
        let system = Arc::new(system);
        let proxy = reqwest::Proxy::custom({
            let system = Arc::clone(&system);
            move |target| system.url(target).cloned()
        });
        Ok(Self {
            client: upstream::upstream_http_client_builder().proxy(proxy.clone()).build()?,
            encoded_client: encoded_client_builder().proxy(proxy).build()?,
            egress: Egress::System(system),
            direct: DirectClients::new()?,
        })
    }

    /// Send a prepared request to the exact target authorized by the application router.
    ///
    /// The caller supplies provider credentials in the prepared request. The returned
    /// status and headers are upstream data and must be filtered before returning them
    /// to an application client.
    pub async fn forward(
        &self,
        authorized_target: &reqwest::Url,
        request: reqwest::Request,
        cancel: CancellationToken,
    ) -> Result<Response, StatusCode> {
        #[cfg(target_os = "macos")]
        if authorized_target.host_str().is_some_and(outbound_system::is_loopback) {
            return forward_with_client(&self.direct.client, authorized_target, request, cancel)
                .await;
        }
        forward_with_client(&self.client, authorized_target, request, cancel).await
    }

    pub(super) async fn forward_ordered(
        &self,
        authorized_target: &reqwest::Url,
        mut request: reqwest::Request,
        wire: HeaderWire,
        cancel: CancellationToken,
        listener: SocketAddr,
    ) -> Result<Response, StatusCode> {
        upstream::validate_target(authorized_target, &request)?;
        let uri: Uri = authorized_target.as_str().parse().map_err(|_| StatusCode::BAD_GATEWAY)?;
        // Leave decoding to the JSON handler so it can enforce the wire and
        // decoded budgets before sending a success status.
        request
            .headers_mut()
            .entry(header::ACCEPT_ENCODING)
            .or_insert(HeaderValue::from_static("zstd,gzip,deflate,br"));
        #[cfg(target_os = "macos")]
        let (client, proxy) = {
            if outbound_system::is_listener(&uri, listener) {
                return Err(StatusCode::BAD_GATEWAY);
            }
            if uri.host().is_some_and(outbound_system::is_loopback) {
                (&self.direct.encoded, None)
            } else {
                match &self.egress {
                    Egress::System(system) => match system.intercept(authorized_target) {
                        Some(proxy) if !outbound_system::is_listener(proxy.uri(), listener) => {
                            (&self.encoded_client, Some(proxy))
                        }
                        _ => (&self.direct.encoded, None),
                    },
                    Egress::Custom(proxy) => {
                        if outbound_system::is_listener(proxy.uri(), listener) {
                            return Err(StatusCode::BAD_GATEWAY);
                        }
                        (&self.encoded_client, Some(proxy))
                    }
                }
            }
        };
        #[cfg(not(target_os = "macos"))]
        let (client, proxy) = {
            let _ = listener;
            match &self.egress {
                Egress::System(matcher) if matcher.intercept(&uri).is_some() => {
                    return forward_with_client(
                        &self.encoded_client,
                        authorized_target,
                        request,
                        cancel,
                    )
                    .await;
                }
                Egress::System(_) => (&self.encoded_client, None),
                Egress::Custom(proxy) => (&self.encoded_client, Some(proxy)),
            }
        };
        if wire.is_empty()
            || proxy
                .is_some_and(|proxy| !matches!(proxy.uri().scheme_str(), Some("http" | "https")))
        {
            return forward_with_client(client, authorized_target, request, cancel).await;
        }
        let method = request.method().clone();
        let mut headers = std::mem::take(request.headers_mut());
        headers.entry(header::ACCEPT).or_insert(HeaderValue::from_static("*/*"));
        let body = request.body_mut().take().ok_or(StatusCode::BAD_GATEWAY)?;
        let mut outgoing = Request::new(body);
        *outgoing.method_mut() = method;
        *outgoing.uri_mut() = uri;
        *outgoing.headers_mut() = headers;

        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(StatusCode::REQUEST_TIMEOUT),
            result = send_ordered(outgoing, wire, proxy, cancel.clone()) => result,
        }
    }
}

fn encoded_client_builder() -> reqwest::ClientBuilder {
    upstream::upstream_http_client_builder_without_total_timeout()
        .retry(reqwest::retry::never())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
}

async fn forward_with_client(
    client: &reqwest::Client,
    authorized_target: &reqwest::Url,
    request: reqwest::Request,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    let mut upstream = upstream::execute(client, authorized_target, request, &cancel).await?;
    let status = upstream.status();
    let headers = std::mem::take(upstream.headers_mut());
    let mut response = Response::new(upstream::response_body(upstream, cancel));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

async fn send_ordered(
    request: Request<reqwest::Body>,
    wire: HeaderWire,
    tunnel: Option<&Intercept>,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    let (parts, body) = request.into_parts();
    let body = body.as_bytes().ok_or(StatusCode::BAD_GATEWAY)?;
    let authority = parts.uri.authority().ok_or(StatusCode::BAD_GATEWAY)?;
    let host = authority.host().trim_start_matches('[').trim_end_matches(']');
    let https = parts.uri.scheme_str() == Some("https");
    let port = authority.port_u16().unwrap_or(if https { 443 } else { 80 });
    let path = parts.uri.path_and_query().map_or("/", |path| path.as_str());
    let mut head = format!(
        "{} {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Length: {}\r\nConnection: close\r\n",
        parts.method,
        body.len(),
    )
    .into_bytes();
    wire.write(&parts.headers, &mut head);
    head.extend_from_slice(b"\r\n");

    let connect_deadline = Instant::now() + upstream::CONNECT_TIMEOUT;
    let socket = timeout_at(connect_deadline, async {
        if let Some(proxy) = tunnel {
            outbound_custom::connect_tunnel(proxy, &parts.uri, tls_connector()?).await
        } else {
            TcpStream::connect((host, port))
                .await
                .map(Either::Left)
                .map_err(|_| StatusCode::BAD_GATEWAY)
        }
    })
    .await
    .map_err(|_| StatusCode::BAD_GATEWAY)??;
    let mut driver = JoinSet::new();
    let response = if https {
        let name = rustls_pki_types::ServerName::try_from(host.to_owned())
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let socket = timeout_at(connect_deadline, tls_connector()?.connect(name, socket))
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        exchange(socket, &head, body, parts.method, &mut driver).await?
    } else {
        exchange(socket, &head, body, parts.method, &mut driver).await?
    };
    Ok(response.map(|body| {
        Body::from_stream(futures_util::stream::unfold(
            (Body::new(body).into_data_stream(), driver, cancel),
            |(mut stream, driver, cancel)| async move {
                let chunk = tokio::select! {
                    biased;
                    () = cancel.cancelled() => return None,
                    chunk = stream.next() => chunk,
                };
                chunk.map(|chunk| (chunk, (stream, driver, cancel)))
            },
        ))
    }))
}

fn tls_connector() -> Result<&'static TlsConnector, StatusCode> {
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

async fn exchange(
    socket: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
    head: &[u8],
    body: &[u8],
    method: axum::http::Method,
    driver: &mut JoinSet<()>,
) -> Result<axum::http::Response<Incoming>, StatusCode> {
    let (reader, mut writer) = split(socket);
    // Hyper parses the response, while the real request uses the ordered
    // wire head. Poll reads during writes so an early response can stop upload.
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(join(reader, sink())))
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
    driver.spawn(async move {
        let _ = connection.await;
    });
    let mut request = Request::new(Body::empty());
    *request.method_mut() = method;
    let response = sender.send_request(request);
    tokio::pin!(response);
    tokio::select! {
        biased;
        response = &mut response => response,
        result = async {
            writer.write_all(head).await?;
            writer.write_all(body).await?;
            writer.flush().await
        } => {
            result.map_err(|_| StatusCode::BAD_GATEWAY)?;
            response.await
        }
    }
    .map_err(|_| StatusCode::BAD_GATEWAY)
}
