//! HTTP execution for a destination and request already authorized by the caller.
use axum::{
    body::Body,
    http::{StatusCode, header},
};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

pub use model_http_client::{
    CONNECT_TIMEOUT, REQUEST_TIMEOUT, upstream_http_client, upstream_http_client_builder,
    upstream_http_client_builder_without_total_timeout,
};

pub async fn execute(
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

pub fn validate_target(
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

pub fn response_body(response: reqwest::Response, cancel: CancellationToken) -> Body {
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
