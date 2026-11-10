//! HTTP execution for a destination and request already authorized by the caller.
use axum::{
    body::Body,
    http::{StatusCode, header},
};
use futures_util::StreamExt;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Common upstream client for relay transports: credentials must never follow redirects.
pub fn upstream_http_client() -> Result<reqwest::Client, reqwest::Error> {
    upstream_http_client_builder().build()
}

pub(crate) fn upstream_http_client_builder() -> reqwest::ClientBuilder {
    upstream_http_client_builder_without_total_timeout().timeout(REQUEST_TIMEOUT)
}

pub(crate) fn upstream_http_client_builder_without_total_timeout() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
}

pub(crate) async fn execute(
    client: &reqwest::Client,
    authorized_target: &reqwest::Url,
    request: reqwest::Request,
    cancel: &CancellationToken,
) -> Result<reqwest::Response, StatusCode> {
    validate_target(authorized_target, &request)?;
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(StatusCode::REQUEST_TIMEOUT),
        response = client.execute(request) => response.map_err(|_| StatusCode::BAD_GATEWAY),
    }
}

pub(crate) fn validate_target(
    authorized_target: &reqwest::Url,
    request: &reqwest::Request,
) -> Result<(), StatusCode> {
    if request.url() != authorized_target || request.headers().contains_key(header::HOST) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(authorized_target.scheme(), "http" | "https")
        || !authorized_target.username().is_empty()
        || authorized_target.password().is_some()
        || authorized_target.fragment().is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}

pub(crate) fn response_body(response: reqwest::Response, cancel: CancellationToken) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        (response.bytes_stream(), cancel),
        |(mut stream, cancel)| async move {
            tokio::select! {
                biased;
                () = cancel.cancelled() => None,
                next = stream.next() => next.map(|value| {
                    let chunk = value.map_err(|_| std::io::Error::other("upstream stream failed"));
                    (chunk, (stream, cancel))
                }),
            }
        },
    ))
}
