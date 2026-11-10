use super::header_wire::HeaderWire;
use crate::model_relay::upstream;
use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode, Uri, header},
    response::Response,
};
use futures_util::StreamExt;
use hyper::body::Incoming;
use hyper_util::{client::proxy::matcher::Matcher, rt::TokioIo};
use rustls_platform_verifier::BuilderVerifierExt;
use std::sync::{Arc, OnceLock};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, join, sink, split},
    net::TcpStream,
    task::JoinSet,
    time::{Instant, timeout_at},
};
use tokio_rustls::TlsConnector;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use tower_http::decompression::Decompression;

/// HTTP transport for the application router, independent of session relay bindings.
pub struct SharedTransport {
    client: reqwest::Client,
    proxies: Matcher,
}

impl SharedTransport {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self { client: upstream::upstream_http_client()?, proxies: Matcher::from_system() })
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
        let mut upstream =
            upstream::execute(&self.client, authorized_target, request, &cancel).await?;
        let status = upstream.status();
        let headers = std::mem::take(upstream.headers_mut());
        let mut response = Response::new(upstream::response_body(upstream, cancel));
        *response.status_mut() = status;
        *response.headers_mut() = headers;
        Ok(response)
    }

    pub(super) async fn forward_ordered(
        &self,
        authorized_target: &reqwest::Url,
        mut request: reqwest::Request,
        wire: HeaderWire,
        cancel: CancellationToken,
    ) -> Result<Response, StatusCode> {
        upstream::validate_target(authorized_target, &request)?;
        let uri: Uri = authorized_target.as_str().parse().map_err(|_| StatusCode::BAD_GATEWAY)?;
        // The established client owns proxy selection and tunneling. Never
        // fall back to a direct connection when its proxy matcher selects one.
        if wire.is_empty() || self.proxies.intercept(&uri).is_some() {
            return self.forward(authorized_target, request, cancel).await;
        }
        let method = request.method().clone();
        let mut headers = std::mem::take(request.headers_mut());
        headers.entry(header::ACCEPT).or_insert(HeaderValue::from_static("*/*"));
        let body = request.body_mut().take().ok_or(StatusCode::BAD_GATEWAY)?;
        let mut outgoing = Request::new(body);
        *outgoing.method_mut() = method;
        *outgoing.uri_mut() = uri;
        *outgoing.headers_mut() = headers;

        let deadline = Instant::now() + upstream::REQUEST_TIMEOUT;
        // Use the same decompression middleware as the existing reqwest client.
        let service = Decompression::new(tower::service_fn(|outgoing: Request<reqwest::Body>| {
            send_ordered(outgoing, wire.clone(), cancel.clone(), deadline)
        }));
        let response = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT),
            result = timeout_at(deadline, service.oneshot(outgoing)) => {
                result.map_err(|_| StatusCode::BAD_GATEWAY)??
            }
        };
        Ok(response.map(Body::new))
    }
}

async fn send_ordered(
    request: Request<reqwest::Body>,
    wire: HeaderWire,
    cancel: CancellationToken,
    deadline: Instant,
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
    let tcp = timeout_at(connect_deadline, TcpStream::connect((host, port)))
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let mut driver = JoinSet::new();
    let response = if https {
        let name = rustls_pki_types::ServerName::try_from(host.to_owned())
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let socket = timeout_at(connect_deadline, tls_connector()?.connect(name, tcp))
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        exchange(socket, &head, body, parts.method, &mut driver).await?
    } else {
        exchange(tcp, &head, body, parts.method, &mut driver).await?
    };
    Ok(response.map(|body| {
        Body::from_stream(futures_util::stream::unfold(
            (Body::new(body).into_data_stream(), driver, cancel),
            move |(mut stream, driver, cancel)| async move {
                let chunk = tokio::select! {
                    biased;
                    () = cancel.cancelled() => return None,
                    result = timeout_at(deadline, stream.next()) => {
                        result.unwrap_or_else(|_| Some(Err(axum::Error::new(
                            std::io::Error::new(std::io::ErrorKind::TimedOut, "upstream body timed out"),
                        ))))
                    }
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
