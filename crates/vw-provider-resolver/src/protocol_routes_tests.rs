use crate::protocol_routes::*;
use vw_shared::provider::protocols::*;

const CAPS: &[RouteCapability] = &[RouteCapability::Text, RouteCapability::Tools];

fn endpoint(protocol: ModelProtocol) -> ProtocolEndpoint {
    serde_json::from_value(serde_json::json!({
        "protocol": protocol, "url": format!("https://custom.example/{protocol:?}"),
        "deployment": "selected-plan", "authentication": {"kind":"bearer"},
        "models": {"kind":"models","ids":["exact-model"]},
        "evidence": [{"kind":"configuration","source":"user","reference":"explicit endpoint","checked_at":"2026-11-11"}]
    })).unwrap()
}

fn binding() -> RouteBinding {
    RouteBinding {
        provider: "unknown-sdk".into(),
        model: "exact-model".into(),
        deployment: "selected-plan".into(),
        account: "selected-account".into(),
    }
}

fn run(
    source: ModelProtocol,
    endpoints: &[ProtocolEndpoint],
    required: &[RouteCapability],
    conversions: &[ConversionSupport<'_>],
) -> Result<ProtocolRoute, RouteError> {
    let selected = binding();
    let grants: Vec<_> = endpoints
        .iter()
        .map(|endpoint| EndpointGrant { binding: &selected, endpoint, capabilities: CAPS })
        .collect();
    select_route(&RouteRequest {
        binding: &binding(),
        source,
        required,
        endpoints: Some(endpoints),
        grants: &grants,
        conversions,
    })
}

#[test]
fn model_protocol_011_native_wins_over_conversion_for_all_sources() {
    for source in [
        ModelProtocol::Chat,
        ModelProtocol::Messages,
        ModelProtocol::Responses,
        ModelProtocol::Gemini,
    ] {
        let other = if source == ModelProtocol::Chat {
            ModelProtocol::Messages
        } else {
            ModelProtocol::Chat
        };
        let endpoints = [endpoint(other), endpoint(source)];
        let conversion = [ConversionSupport { source, target: other, capabilities: CAPS }];
        let route = run(source, &endpoints, CAPS, &conversion).unwrap();
        assert!(!route.conversion_required);
        assert_eq!(route.endpoint, endpoints[1]);
        assert_eq!(route.binding, binding());
        assert_eq!(route.source, source);
        assert_eq!(route.target, source);
    }
}

#[test]
fn model_protocol_011_only_declared_direction_and_capabilities_are_executable() {
    for source in [
        ModelProtocol::Chat,
        ModelProtocol::Messages,
        ModelProtocol::Responses,
        ModelProtocol::Gemini,
    ] {
        let target = if source == ModelProtocol::Chat {
            ModelProtocol::Responses
        } else {
            ModelProtocol::Chat
        };
        let endpoints = [endpoint(target)];
        assert!(run(source, &endpoints, CAPS, &[]).is_err());
        let conversion = [ConversionSupport { source, target, capabilities: CAPS }];
        let route = run(source, &endpoints, CAPS, &conversion).unwrap();
        assert!(route.conversion_required);
        assert_eq!(route.endpoint, endpoints[0]);
        assert!(run(source, &endpoints, &[RouteCapability::ThinkingReplay], &conversion).is_err());
        let limited =
            [ConversionSupport { source, target, capabilities: &[RouteCapability::Text] }];
        assert!(run(source, &endpoints, CAPS, &limited).is_err());
    }
}

#[test]
fn model_protocol_011_only_native_messages_responses_gemini_need_no_chat() {
    for source in [ModelProtocol::Messages, ModelProtocol::Responses, ModelProtocol::Gemini] {
        let endpoints = [endpoint(source)];
        assert_eq!(run(source, &endpoints, CAPS, &[]).unwrap().endpoint, endpoints[0]);
    }
}

#[test]
fn model_protocol_011_rejects_unknown_and_explicit_empty_without_sdk_guessing() {
    for (endpoints, reason) in
        [(None, RouteRejection::UnknownProtocols), (Some(&[][..]), RouteRejection::NoEndpoints)]
    {
        let error = select_route(&RouteRequest {
            binding: &binding(),
            source: ModelProtocol::Chat,
            required: CAPS,
            endpoints,
            grants: &[],
            conversions: &[],
        })
        .unwrap_err();
        assert_eq!(error.reasons, vec![reason]);
    }
}

#[test]
fn model_protocol_011_explicit_custom_url_and_authentication_remain_bound() {
    let mut candidate = endpoint(ModelProtocol::Messages);
    candidate.url = "https://private.example/explicit-operation".into();
    candidate.authentication =
        ProtocolAuthentication::ApiKeyHeader { name: "x-private-key".into() };
    let endpoints = [candidate.clone()];
    let route = run(ModelProtocol::Messages, &endpoints, CAPS, &[]).unwrap();
    assert_eq!(route.endpoint, candidate);
    assert_eq!(route.binding, binding());
    assert!(
        select_route(&RouteRequest {
            binding: &binding(),
            source: ModelProtocol::Messages,
            required: CAPS,
            endpoints: Some(&endpoints),
            grants: &[],
            conversions: &[]
        })
        .unwrap_err()
        .reasons
        .contains(&RouteRejection::EndpointNotGranted)
    );
}

#[test]
fn model_protocol_011_rejects_wrong_model_deployment_template_and_url() {
    for (field, value, reason) in [
        ("model", "other-model", RouteRejection::ModelNotApplicable),
        ("deployment", "other-plan", RouteRejection::WrongDeployment),
        ("url", "https://${REGION}.example/messages", RouteRejection::MissingDeploymentParameters),
        ("url", "file:///tmp/messages", RouteRejection::InvalidEndpoint),
        ("url", "https://user:secret@example.com/messages", RouteRejection::InvalidEndpoint),
    ] {
        let mut candidate = endpoint(ModelProtocol::Messages);
        match field {
            "model" => candidate.models = ProtocolModelScope::Models { ids: vec![value.into()] },
            "deployment" => candidate.deployment = value.into(),
            _ => candidate.url = value.into(),
        }
        let error = run(ModelProtocol::Messages, &[candidate], CAPS, &[]).unwrap_err();
        assert!(error.reasons.contains(&reason), "{error:?}");
    }
}

#[test]
fn model_protocol_011_multiple_native_candidates_require_disambiguation() {
    let first = endpoint(ModelProtocol::Chat);
    let mut second = first.clone();
    second.url = "https://other.example/chat".into();
    assert!(
        run(ModelProtocol::Chat, &[first.clone(), second], CAPS, &[])
            .unwrap_err()
            .reasons
            .contains(&RouteRejection::AmbiguousNativeEndpoints)
    );
    assert_eq!(
        run(ModelProtocol::Chat, &[first.clone(), first.clone()], CAPS, &[]).unwrap().endpoint,
        first
    );
}

#[test]
fn model_protocol_011_grants_cannot_transfer_account_or_authentication() {
    let selected = binding();
    let mut other = selected.clone();
    other.account = "other-account".into();
    let endpoints = [endpoint(ModelProtocol::Chat)];
    let grants = [EndpointGrant { binding: &other, endpoint: &endpoints[0], capabilities: CAPS }];
    let request = RouteRequest {
        binding: &selected,
        source: ModelProtocol::Chat,
        required: CAPS,
        endpoints: Some(&endpoints),
        grants: &grants,
        conversions: &[],
    };
    assert_eq!(
        select_route(&request).unwrap_err().reasons,
        vec![RouteRejection::EndpointNotGranted]
    );
    let mut different_auth = endpoints[0].clone();
    different_auth.authentication = ProtocolAuthentication::OAuthBearer {};
    let grants =
        [EndpointGrant { binding: &selected, endpoint: &different_auth, capabilities: CAPS }];
    assert_eq!(
        select_route(&RouteRequest { grants: &grants, ..request }).unwrap_err().reasons,
        vec![RouteRejection::EndpointNotGranted]
    );
}

#[test]
fn model_protocol_011_unsupported_authentication_and_missing_evidence_are_rejected() {
    let mut candidate = endpoint(ModelProtocol::Chat);
    candidate.authentication =
        ProtocolAuthentication::Unsupported { reason: "proprietary signing".into() };
    assert_eq!(
        run(ModelProtocol::Chat, &[candidate.clone()], CAPS, &[]).unwrap_err().reasons,
        vec![RouteRejection::UnsupportedAuthentication]
    );
    candidate.authentication = ProtocolAuthentication::Bearer {};
    candidate.evidence.clear();
    assert_eq!(
        run(ModelProtocol::Chat, &[candidate], CAPS, &[]).unwrap_err().reasons,
        vec![RouteRejection::MissingEvidence]
    );
}

#[test]
fn model_protocol_011_grants_require_one_complete_capability_check() {
    let selected = binding();
    let endpoints = [endpoint(ModelProtocol::Chat)];
    let grants = [
        EndpointGrant {
            binding: &selected,
            endpoint: &endpoints[0],
            capabilities: &[RouteCapability::Text],
        },
        EndpointGrant {
            binding: &selected,
            endpoint: &endpoints[0],
            capabilities: &[RouteCapability::Tools],
        },
    ];
    assert_eq!(
        select_route(&RouteRequest {
            binding: &selected,
            source: ModelProtocol::Chat,
            required: CAPS,
            endpoints: Some(&endpoints),
            grants: &grants,
            conversions: &[]
        })
        .unwrap_err()
        .reasons,
        vec![RouteRejection::CapabilityUnavailable]
    );
}

#[test]
fn model_protocol_011_all_conversion_pairs_require_explicit_caller_support() {
    let protocols = [
        ModelProtocol::Chat,
        ModelProtocol::Messages,
        ModelProtocol::Responses,
        ModelProtocol::Gemini,
    ];
    for source in protocols {
        for target in protocols {
            if source == target {
                continue;
            }
            let endpoints = [endpoint(target)];
            assert!(run(source, &endpoints, CAPS, &[]).is_err());
            let conversion = [ConversionSupport { source, target, capabilities: CAPS }];
            let route = run(source, &endpoints, CAPS, &conversion).unwrap();
            assert_eq!((route.source, route.target), (source, target));
            assert_eq!(route.binding, binding());
        }
    }
}

#[test]
fn model_protocol_011_deployment_scope_disambiguates_native_candidates() {
    let mut other = endpoint(ModelProtocol::Chat);
    other.deployment = "other-region".into();
    let selected = endpoint(ModelProtocol::Chat);
    assert_eq!(
        run(ModelProtocol::Chat, &[other, selected.clone()], CAPS, &[]).unwrap().endpoint,
        selected
    );
}
