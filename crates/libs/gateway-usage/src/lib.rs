//! Canonical usage ledger and host maintenance endpoints.
mod usage;
mod read;
mod scan;
mod events;
mod prices;
pub mod host;
pub use host::Host;

pub fn router(host: Host) -> axum::Router {
    read::router(host).merge(scan::router(host)).merge(events::router(host)).merge(prices::router(host))
}

async fn without_query(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    if request.uri().query().is_some_and(|query| !query.is_empty()) {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod local_scope_tests;
