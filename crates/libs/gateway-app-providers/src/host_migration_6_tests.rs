//! Provider domain scenarios registered by their owning crate.
#[cfg(target_os = "macos")]
use crate::*;

#[cfg(target_os = "macos")]
async fn publication(mut request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    request.extensions_mut().insert(gateway_app_routing::server::provider_publication_host::control(None));
    next.run(request).await
}

#[cfg(target_os = "macos")]
mod claude_tests;
#[cfg(target_os = "macos")]
mod codex_catalog_tests;
#[cfg(target_os = "macos")]
mod claude_desktop_tests;
#[cfg(target_os = "macos")]
mod agy_tests;
#[cfg(target_os = "macos")]
mod catalog_tests;
mod basic_tests;
mod create_tests;

fn record_router() -> axum::Router {
    crate::router()
        .merge(crate::basic::router())
        .merge(crate::duplicate::router())
}
#[cfg(target_os = "macos")]
mod codex_tests;
#[cfg(target_os = "macos")]
mod delete_tests;

#[cfg(target_os = "macos")]
mod api_security_tests;
