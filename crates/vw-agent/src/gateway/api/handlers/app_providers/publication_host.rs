//! Strongly typed adapter to the existing gateway publication transaction.
#![cfg(target_os = "macos")]
use super::*;
use super::super::app_routing::RoutingControl;
// Concrete publication implementation belongs to gateway-app-routing.
pub(super) use gateway_app_routing::server::provider_publication_host::control;
pub(super) fn input(input: PublicationInput) -> gateway_app_providers::PublicationInput {
    gateway_app_providers::PublicationInput { query: input.query, routing: Some(control(input.routing)) }
}
pub(super) async fn inject(routing: Option<Extension<RoutingControl>>,
    mut request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    request.extensions_mut().insert(control(routing.map(|Extension(c)| c)));
    next.run(request).await
}
