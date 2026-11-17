//! Grant only the selected deployment's origin, then use the shared pure selector.
use crate::provider::provider::Model;
use axum::http::StatusCode;
#[cfg(any(test, target_arch = "wasm32"))]
use provider_protocol_routes::RouteCapability;
use model_protocols::ModelProtocol;

pub(crate) fn configured_protocol(model: &Model) -> Result<ModelProtocol, StatusCode> {
    if super::responses::is_native(model) {
        return Ok(ModelProtocol::Responses);
    }
    configured_api_protocol(&model.api)
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_transport::routing_selection::client_protocol;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_transport::routing_selection::select;

/// ACP has the same selected API identity but does not carry catalog pricing or media.
/// The caller grants only the capabilities it actually needs at startup.

/// Capabilities needed for route selection; protocol validators still reject
/// unsupported fields and replay state before any network side effect.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_transport::routing_selection::required;

/// Complete conversion directions currently implemented by ProviderRelayBackend.

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use model_transport::routing_selection::configured_api_protocol;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use model_transport::routing_selection::{Execution, configured_api_protocol, default_authentication, relay_conversions, select_api};
