use crate::model_relay::upstream;
use axum::{http::StatusCode, response::Response};
use tokio_util::sync::CancellationToken;

/// HTTP transport for the application router, independent of session relay bindings.
pub struct SharedTransport {
    client: reqwest::Client,
}

impl SharedTransport {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self { client: upstream::upstream_http_client()? })
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
}
