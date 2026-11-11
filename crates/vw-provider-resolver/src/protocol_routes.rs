//! Pure, pre-dispatch selection. Callers retain credentials and enforce access before
//! granting an exact endpoint. A route is immutable for the lifetime of an attempt;
//! transport failures never authorize selecting another model, account or endpoint.
use vw_shared::provider::protocols::{ModelProtocol, ProtocolEndpoint};

/// Capabilities actually required by a request, including state carried between turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCapability {
    Text,
    Tools,
    Thinking,
    ThinkingReplay,
    Streaming,
    StructuredOutput,
    Image,
    Audio,
    Video,
    Pdf,
}

/// Already selected identity. The account identifier is opaque, never a credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteBinding {
    pub provider: String,
    pub model: String,
    pub deployment: String,
    pub account: String,
}

/// Caller-validated endpoint access, authentication implementation, and model limits.
/// Construct only after checking this exact URL/authentication against the selected
/// binding. Cross-origin grants require service evidence AND the existing access
/// checks; catalog evidence alone is not authorization. Contains no credential values.
#[derive(Debug)]
pub struct EndpointGrant<'a> {
    pub binding: &'a RouteBinding,
    pub endpoint: &'a ProtocolEndpoint,
    pub capabilities: &'a [RouteCapability],
}

/// One direction implemented by the caller, including request, response and stream
/// conversion. Do not advertise combinations or state that the converter rejects.
#[derive(Debug)]
pub struct ConversionSupport<'a> {
    pub source: ModelProtocol,
    pub target: ModelProtocol,
    pub capabilities: &'a [RouteCapability],
}

/// No SDK inference: unknown facts and explicit empty declarations remain distinct.
#[derive(Debug)]
pub struct RouteRequest<'a> {
    pub binding: &'a RouteBinding,
    pub source: ModelProtocol,
    pub required: &'a [RouteCapability],
    pub endpoints: Option<&'a [ProtocolEndpoint]>,
    pub grants: &'a [EndpointGrant<'a>],
    pub conversions: &'a [ConversionSupport<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolRoute {
    pub binding: RouteBinding,
    pub endpoint: ProtocolEndpoint,
    pub source: ModelProtocol,
    pub target: ModelProtocol,
    pub conversion_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteRejection {
    InvalidBinding,
    UnknownProtocols,
    NoEndpoints,
    WrongDeployment,
    ModelNotApplicable,
    MissingDeploymentParameters,
    InvalidEndpoint,
    MissingEvidence,
    EndpointNotGranted,
    UnsupportedAuthentication,
    CapabilityUnavailable,
    ConversionUnavailable,
    AmbiguousNativeEndpoints,
}

/// Failure reasons follow catalog order, without credential values or request payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteError {
    pub reasons: Vec<RouteRejection>,
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "no executable protocol route: {:?}", self.reasons)
    }
}

impl std::error::Error for RouteError {}

/// Select once before dispatch. Native candidates take precedence over conversions;
/// equally executable native endpoints must be disambiguated by the caller's binding.
pub fn select_route(request: &RouteRequest<'_>) -> Result<ProtocolRoute, RouteError> {
    let fail = |reason| RouteError { reasons: vec![reason] };
    let binding = request.binding;
    if [&binding.provider, &binding.model, &binding.deployment, &binding.account]
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(fail(RouteRejection::InvalidBinding));
    }
    let endpoints = request.endpoints.ok_or_else(|| fail(RouteRejection::UnknownProtocols))?;
    if endpoints.is_empty() {
        return Err(fail(RouteRejection::NoEndpoints));
    }
    let mut reasons = Vec::new();
    let mut native = Vec::new();
    let mut converted = Vec::new();
    for endpoint in endpoints {
        if let Err(reason) = eligible_endpoint(request, endpoint) {
            reasons.push(reason);
            continue;
        }
        if endpoint.protocol == request.source {
            if !native.contains(&endpoint) {
                native.push(endpoint);
            }
        } else if request.conversions.iter().any(|conversion| {
            conversion.source == request.source
                && conversion.target == endpoint.protocol
                && covers(conversion.capabilities, request.required)
        }) {
            converted.push(endpoint);
        } else {
            reasons.push(RouteRejection::ConversionUnavailable);
        }
    }
    if native.len() > 1 {
        return Err(fail(RouteRejection::AmbiguousNativeEndpoints));
    }
    let endpoint = native.first().or_else(|| converted.first()).ok_or(RouteError { reasons })?;
    Ok(ProtocolRoute {
        binding: binding.clone(),
        endpoint: (*endpoint).clone(),
        source: request.source,
        target: endpoint.protocol,
        conversion_required: endpoint.protocol != request.source,
    })
}

fn covers(available: &[RouteCapability], required: &[RouteCapability]) -> bool {
    required.iter().all(|capability| available.contains(capability))
}

fn eligible_endpoint(
    request: &RouteRequest<'_>,
    endpoint: &ProtocolEndpoint,
) -> Result<(), RouteRejection> {
    if endpoint.url.contains(['{', '}', '$']) || endpoint.deployment.contains(['{', '}', '$']) {
        return Err(RouteRejection::MissingDeploymentParameters);
    }
    if endpoint.deployment != request.binding.deployment {
        return Err(RouteRejection::WrongDeployment);
    }
    if !endpoint.models.contains(&request.binding.model) {
        return Err(RouteRejection::ModelNotApplicable);
    }
    let url = reqwest::Url::parse(&endpoint.url).map_err(|_| RouteRejection::InvalidEndpoint)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(RouteRejection::InvalidEndpoint);
    }
    if endpoint.evidence.is_empty()
        || endpoint.evidence.iter().any(|evidence| {
            evidence.source.trim().is_empty()
                || evidence.reference.trim().is_empty()
                || evidence.checked_at.trim().is_empty()
        })
    {
        return Err(RouteRejection::MissingEvidence);
    }
    if matches!(
        endpoint.authentication,
        vw_shared::provider::protocols::ProtocolAuthentication::Unsupported { .. }
    ) {
        return Err(RouteRejection::UnsupportedAuthentication);
    }
    let grants: Vec<_> = request
        .grants
        .iter()
        .filter(|grant| grant.endpoint == endpoint && grant.binding == request.binding)
        .collect();
    if grants.is_empty() {
        return Err(RouteRejection::EndpointNotGranted);
    }
    // One complete grant must cover the request. Never union grants belonging to
    // different authorization/capability checks into a synthetic permission.
    if !grants.iter().any(|grant| covers(grant.capabilities, request.required)) {
        return Err(RouteRejection::CapabilityUnavailable);
    }
    Ok(())
}
