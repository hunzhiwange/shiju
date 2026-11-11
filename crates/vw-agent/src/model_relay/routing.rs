//! Grant only the selected deployment's origin, then use the shared pure selector.
use super::generation::GenerationTransport;
use crate::provider::provider::Model;
use axum::http::StatusCode;
use vw_provider_resolver::protocol_routes::{
    ConversionSupport, EndpointGrant, ProtocolRoute, RouteBinding, RouteCapability, RouteRequest,
    select_route,
};
use vw_shared::provider::protocols::{
    ModelProtocol, ProtocolAuthentication, ProtocolEndpoint, ProtocolEvidence,
    ProtocolEvidenceKind, ProtocolModelScope,
};

#[derive(Clone, Copy)]
pub(crate) enum Execution {
    Relay,
    Aisdk,
}

pub(crate) fn configured_protocol(model: &Model) -> Result<ModelProtocol, StatusCode> {
    if super::responses::is_native(model) {
        return Ok(ModelProtocol::Responses);
    }
    configured_api_protocol(&model.api)
}

pub(crate) fn configured_api_protocol(
    api: &crate::provider::provider::ApiInfo,
) -> Result<ModelProtocol, StatusCode> {
    match api.adapter.trim() {
        "openai"
        | "openai-compatible"
        | "acp"
        | "agent-client-protocol"
        | "agent_client_protocol" => {
            if reqwest::Url::parse(&api.url)
                .is_ok_and(|url| url.path().trim_end_matches('/').ends_with("/responses"))
            {
                Ok(ModelProtocol::Responses)
            } else {
                Ok(ModelProtocol::Chat)
            }
        }
        "anthropic" => Ok(ModelProtocol::Messages),
        "openai-responses" => Ok(ModelProtocol::Responses),
        "google" => Ok(ModelProtocol::Gemini),
        _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
}

pub(crate) fn select(
    model: &Model,
    source: ModelProtocol,
    required: &[RouteCapability],
    conversions: &[ConversionSupport<'_>],
    execution: Execution,
) -> Result<ProtocolRoute, StatusCode> {
    // Native catalogs preserve unknown media/tool declarations. Their existing
    // execution gate rejects explicit Unsupported values before SDK conversion.
    let catalog = model.capabilities.catalog_execution.as_ref();
    let input = match catalog {
        Some(catalog) => catalog.input.as_ref(),
        None => Some(&model.capabilities.input),
    };
    let available: Vec<_> = required
        .iter()
        .copied()
        .filter(|capability| match capability {
            RouteCapability::Tools => catalog
                .map_or(model.capabilities.toolcall, |catalog| !catalog.tool_calls_unsupported),
            RouteCapability::Thinking | RouteCapability::ThinkingReplay => {
                model.capabilities.reasoning
            }
            RouteCapability::Image => input.is_none_or(|input| input.image),
            RouteCapability::Audio => input.is_none_or(|input| input.audio),
            RouteCapability::Video => input.is_none_or(|input| input.video),
            RouteCapability::Pdf => input.is_none_or(|input| input.pdf),
            _ => true,
        })
        .collect();
    select_api(&model.api, &model.provider_id, source, required, &available, conversions, execution)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}

/// ACP has the same selected API identity but does not carry catalog pricing or media.
/// The caller grants only the capabilities it actually needs at startup.
pub(crate) fn select_api(
    api: &crate::provider::provider::ApiInfo,
    provider: &str,
    source: ModelProtocol,
    required: &[RouteCapability],
    available: &[RouteCapability],
    conversions: &[ConversionSupport<'_>],
    execution: Execution,
) -> Result<ProtocolRoute, vw_provider_resolver::protocol_routes::RouteError> {
    use vw_provider_resolver::protocol_routes::{RouteError, RouteRejection};
    let invalid = || RouteError { reasons: vec![RouteRejection::InvalidEndpoint] };
    let base = reqwest::Url::parse(api.url.trim()).map_err(|_| invalid())?;
    if !matches!(base.scheme(), "http" | "https")
        || !base.username().is_empty()
        || base.password().is_some()
        || base.fragment().is_some()
    {
        return Err(invalid());
    }
    let fallback;
    let endpoints = match api.protocols.as_deref() {
        Some(endpoints) => endpoints,
        None => {
            // Preserve the explicitly configured adapter/URL contract. Unknown SDK
            // identities never acquire an inferred OpenAI implementation.
            let protocol = configured_api_protocol(api)
                .map_err(|_| RouteError { reasons: vec![RouteRejection::UnknownProtocols] })?;
            fallback = vec![ProtocolEndpoint {
                protocol,
                url: protocol.url(&api.url, &api.id, false).map_err(|_| invalid())?.to_string(),
                deployment: api.url.clone(),
                authentication: default_authentication(protocol),
                models: ProtocolModelScope::Models { ids: vec![api.id.clone()] },
                evidence: vec![ProtocolEvidence {
                    kind: ProtocolEvidenceKind::Configuration,
                    source: "selected model API configuration".into(),
                    reference: "api.url and api.adapter".into(),
                    checked_at: chrono::Utc::now().date_naive().to_string(),
                }],
            }];
            &fallback
        }
    };
    if endpoints.is_empty() {
        return Err(RouteError { reasons: vec![RouteRejection::NoEndpoints] });
    }
    let same_origin = |endpoint: &&ProtocolEndpoint| {
        reqwest::Url::parse(&endpoint.url).is_ok_and(|url| url.origin() == base.origin())
    };
    let candidates: Vec<_> = endpoints.iter().filter(same_origin).collect();
    // An exact operation URL disambiguates deployments. A base URL can only
    // authorize one deployment on this origin, never union multiple accounts.
    let anchors: Vec<_> =
        candidates.iter().copied().filter(|endpoint| endpoint.url == base.as_str()).collect();
    let scope = if anchors.is_empty() { &candidates } else { &anchors };
    let deployment = scope
        .first()
        .ok_or_else(|| RouteError { reasons: vec![RouteRejection::EndpointNotGranted] })?
        .deployment
        .clone();
    if scope.iter().any(|endpoint| endpoint.deployment != deployment) {
        return Err(RouteError { reasons: vec![RouteRejection::WrongDeployment] });
    }
    let binding = RouteBinding {
        provider: provider.to_owned(),
        model: api.id.clone(),
        deployment,
        account: provider.to_owned(),
    };
    let grants: Vec<_> = candidates
        .into_iter()
        .filter(|endpoint| match execution {
            Execution::Relay => supported_authentication(&endpoint.authentication),
            Execution::Aisdk => {
                let suffix = match endpoint.protocol {
                    ModelProtocol::Chat => "/chat/completions",
                    ModelProtocol::Messages => "/messages",
                    ModelProtocol::Responses => "/responses",
                    ModelProtocol::Gemini => return false,
                };
                endpoint.authentication == default_authentication(endpoint.protocol)
                    && endpoint.url.ends_with(suffix)
            }
        })
        .map(|endpoint| EndpointGrant { binding: &binding, endpoint, capabilities: available })
        .collect();
    select_route(&RouteRequest {
        binding: &binding,
        source,
        required,
        endpoints: Some(endpoints),
        grants: &grants,
        conversions,
    })
}

pub(crate) fn default_authentication(protocol: ModelProtocol) -> ProtocolAuthentication {
    match protocol {
        ModelProtocol::Messages => {
            ProtocolAuthentication::ApiKeyHeader { name: "x-api-key".into() }
        }
        ModelProtocol::Gemini => {
            ProtocolAuthentication::ApiKeyHeader { name: "x-goog-api-key".into() }
        }
        _ => ProtocolAuthentication::Bearer {},
    }
}

fn supported_authentication(authentication: &ProtocolAuthentication) -> bool {
    match authentication {
        ProtocolAuthentication::None {}
        | ProtocolAuthentication::Bearer {}
        | ProtocolAuthentication::OAuthBearer {} => true,
        ProtocolAuthentication::ApiKeyHeader { name } => {
            matches!(name.to_ascii_lowercase().as_str(), "x-api-key" | "x-goog-api-key" | "api-key")
        }
        _ => false,
    }
}

/// Capabilities needed for route selection; protocol validators still reject
/// unsupported fields and replay state before any network side effect.
pub(crate) fn required(body: &serde_json::Value, streaming: bool) -> Vec<RouteCapability> {
    use RouteCapability as C;
    let mut result = vec![C::Text];
    if streaming {
        result.push(C::Streaming);
    }
    fn add(result: &mut Vec<C>, capability: C) {
        if !result.contains(&capability) {
            result.push(capability);
        }
    }
    for key in ["tools", "tool_choice", "functions", "function_call"] {
        if body.get(key).is_some_and(|value| {
            !value.is_null() && value != &serde_json::json!([]) && value != "none"
        }) {
            add(&mut result, C::Tools);
        }
    }
    if body.get("thinking").is_some_and(|value| {
        !value.is_null() && value != &serde_json::Value::Bool(false) && value["type"] != "disabled"
    }) || body.get("reasoning").is_some_and(|value| {
        !value.is_null()
            && value != &serde_json::Value::Bool(false)
            && value != "none"
            && value["effort"] != "none"
            && value["enabled"] != false
    }) || body.get("reasoning_effort").is_some_and(|value| !value.is_null() && value != "none")
    {
        add(&mut result, C::Thinking);
    }
    if ["response_format", "output_config"].iter().any(|key| body.get(key).is_some()) {
        add(&mut result, C::StructuredOutput);
    }
    let messages = body
        .as_array()
        .or_else(|| body["messages"].as_array())
        .or_else(|| body["input"].as_array());
    for message in messages.into_iter().flatten() {
        if message.get("tool_calls").is_some()
            || matches!(message["role"].as_str(), Some("tool" | "function"))
            || matches!(message["type"].as_str(), Some("function_call" | "function_call_output"))
        {
            add(&mut result, C::Tools);
        }
        if message
            .get("reasoning_content")
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
            || message["type"] == "reasoning"
        {
            add(&mut result, C::ThinkingReplay);
        }
        for part in message["content"].as_array().into_iter().flatten() {
            match part["type"].as_str() {
                Some("thinking" | "redacted_thinking") => add(&mut result, C::ThinkingReplay),
                Some("tool_use" | "tool_result") => add(&mut result, C::Tools),
                Some("image" | "image_url" | "input_image") => add(&mut result, C::Image),
                Some("input_audio") => add(&mut result, C::Audio),
                Some("document") => add(&mut result, C::Pdf),
                _ => {}
            }
        }
    }
    result
}

/// Complete conversion directions currently implemented by ProviderRelayBackend.
pub(crate) fn relay_conversions(codex: bool) -> [ConversionSupport<'static>; 3] {
    use ModelProtocol as P;
    use RouteCapability as C;
    const BASIC: &[C] = &[C::Text, C::Tools, C::Streaming, C::Image];
    const RESPONSES: &[C] = &[C::Text, C::Tools, C::Streaming, C::Image, C::StructuredOutput];
    const CODEX: &[C] = &[
        C::Text,
        C::Tools,
        C::Streaming,
        C::Image,
        C::Thinking,
        C::ThinkingReplay,
        C::StructuredOutput,
    ];
    [
        ConversionSupport { source: P::Messages, target: P::Chat, capabilities: RESPONSES },
        ConversionSupport {
            source: P::Responses,
            target: P::Chat,
            capabilities: if codex { CODEX } else { RESPONSES },
        },
        ConversionSupport { source: P::Gemini, target: P::Chat, capabilities: BASIC },
    ]
}
