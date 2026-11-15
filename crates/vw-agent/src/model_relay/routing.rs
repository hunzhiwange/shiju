//! Grant only the selected deployment's origin, then use the shared pure selector.
use crate::provider::provider::Model;
use axum::http::StatusCode;
use provider_protocol_routes::RouteCapability;
use model_protocols::ModelProtocol;

pub(crate) fn configured_protocol(model: &Model) -> Result<ModelProtocol, StatusCode> {
    if super::responses::is_native(model) {
        return Ok(ModelProtocol::Responses);
    }
    configured_api_protocol(&model.api)
}

/// Select a client's wire format from an explicit HTTP contract when its original
/// SDK identity is otherwise unknown. This never relabels that SDK or authorizes
/// a request: launch_route rechecks the real owner and complete capabilities.
pub(crate) fn client_protocol(
    api: &crate::provider::provider::ApiInfo,
) -> Result<ModelProtocol, StatusCode> {
    if let Ok(protocol) = configured_api_protocol(api) {
        return Ok(protocol);
    }
    let endpoints = api.protocols.as_ref().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut preferred = Vec::new();
    for endpoint in endpoints.iter().filter(|endpoint| endpoint.url == api.url) {
        if !preferred.contains(&endpoint.protocol) {
            preferred.push(endpoint.protocol);
        }
    }
    for protocol in [
        ModelProtocol::Chat,
        ModelProtocol::Messages,
        ModelProtocol::Responses,
        ModelProtocol::Gemini,
    ] {
        if !preferred.contains(&protocol) {
            preferred.push(protocol);
        }
    }
    let capabilities = [RouteCapability::Text];
    for protocol in preferred {
        if select_api(
            api,
            "client-wire-contract",
            protocol,
            &capabilities,
            &capabilities,
            &[],
            Execution::Relay,
        )
        .is_ok()
        {
            return Ok(protocol);
        }
    }
    Err(StatusCode::UNPROCESSABLE_ENTITY)
}

pub(crate) use model_transport::routing_selection::select;

/// ACP has the same selected API identity but does not carry catalog pricing or media.
/// The caller grants only the capabilities it actually needs at startup.

/// Capabilities needed for route selection; protocol validators still reject
/// unsupported fields and replay state before any network side effect.
pub(crate) use model_transport::routing_selection::required;

/// Complete conversion directions currently implemented by ProviderRelayBackend.

pub(crate) use model_transport::routing_selection::{Execution, configured_api_protocol, default_authentication, relay_conversions, select_api};
