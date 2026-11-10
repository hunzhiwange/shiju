//! HTTP execution for a destination and request already authorized by the caller.
use axum::{
    body::Body,
    http::{StatusCode, header},
};
use futures_util::StreamExt;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(super) const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Common upstream client for relay transports: credentials must never follow redirects.
pub fn upstream_http_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(REQUEST_TIMEOUT)
        .build()
}

pub(crate) async fn execute(
    client: &reqwest::Client,
    authorized_target: &reqwest::Url,
    request: reqwest::Request,
    cancel: &CancellationToken,
) -> Result<reqwest::Response, StatusCode> {
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
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(StatusCode::REQUEST_TIMEOUT),
        response = client.execute(request) => response.map_err(|_| StatusCode::BAD_GATEWAY),
    }
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
