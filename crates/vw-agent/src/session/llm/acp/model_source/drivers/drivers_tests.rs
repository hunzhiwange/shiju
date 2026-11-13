use super::*;
use crate::{
    config::schema::AcpAgentConfig, model_relay::RelayIdentity, provider::provider::ApiInfo,
};
use vw_shared::provider::protocols::{ModelProtocol as P, ProtocolEndpoint};

fn profile(adapter: &str) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig::default(),
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session:codex".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_api = Some(ApiInfo {
        id: "selected-model".into(),
        url: "https://example.invalid/v1".into(),
        adapter: adapter.into(),
        protocols: None,
    });
    profile
}

fn endpoint(protocol: P) -> ProtocolEndpoint {
    use vw_shared::provider::protocols::*;
    ProtocolEndpoint {
        protocol,
        url: match protocol {
            P::Chat => "https://example.invalid/v1/chat/completions",
            P::Messages => "https://example.invalid/v1/messages",
            P::Responses => "https://example.invalid/v1/responses",
            P::Gemini => {
                "https://example.invalid/v1beta/models/selected-model:streamGenerateContent"
            }
        }
        .into(),
        deployment: "selected".into(),
        authentication: crate::model_relay::routing::default_authentication(protocol),
        models: ProtocolModelScope::Models { ids: vec!["selected-model".into()] },
        evidence: vec![ProtocolEvidence {
            kind: ProtocolEvidenceKind::Configuration,
            source: "fixture".into(),
            reference: "explicit deployment".into(),
            checked_at: "2026-11-11".into(),
        }],
    }
}

#[test]
fn model_protocol_013_registered_drivers_declare_actual_wire_protocols() {
    let api = profile("openai-compatible").model_api.clone().unwrap();
    for (agent, expected) in [
        ("auggie", P::Chat),
        ("qoder", P::Chat),
        ("cursor", P::Chat),
        ("claude", P::Messages),
        ("codex", P::Responses),
        ("kiro", P::Responses),
        ("gemini", P::Gemini),
        ("codebuddy", P::Chat),
        ("droid", P::Chat),
        ("goose", P::Chat),
        ("hermes", P::Chat),
        ("iflow", P::Chat),
        ("kilocode", P::Chat),
        ("openclaw", P::Chat),
        ("pi", P::Chat),
        ("qwen", P::Chat),
        ("snow", P::Chat),
        ("opencode", P::Chat),
        ("kimi", P::Chat),
        ("copilot", P::Chat),
        ("vibe", P::Chat),
        ("trae", P::Chat),
    ] {
        assert_eq!(source_protocol(agent, &api).unwrap(), expected, "{agent}");
    }
    assert!(source_protocol("unknown", &api).is_err());
    for (adapter, expected) in
        [("anthropic", P::Messages), ("openai-responses", P::Responses), ("google", P::Gemini)]
    {
        let api = profile(adapter).model_api.clone().unwrap();
        for agent in ["snow", "opencode", "kimi"] {
            assert_eq!(source_protocol(agent, &api).unwrap(), expected, "{agent} {adapter}");
        }
    }
}

#[test]
fn model_protocol_013_four_sources_prefer_catalog_native_and_share_relay_conversions() {
    for (agent, source) in
        [("claude", P::Messages), ("codex", P::Responses), ("gemini", P::Gemini), ("qwen", P::Chat)]
    {
        let mut launch = profile("openai-compatible");
        let endpoints = if source == P::Chat {
            vec![endpoint(P::Chat)]
        } else {
            vec![endpoint(P::Chat), endpoint(source)]
        };
        launch.model_api.as_mut().unwrap().protocols = Some(endpoints);
        let route = launch_route(agent, &launch).unwrap();
        assert_eq!(route.source, source);
        assert_eq!(route.target, source);
        assert!(!route.conversion_required);
        launch.model_api.as_mut().unwrap().protocols = Some(vec![endpoint(P::Chat)]);
        let route = launch_route(agent, &launch).unwrap();
        assert_eq!(route.target, P::Chat);
        assert_eq!(route.conversion_required, source != P::Chat);
        launch.model_api.as_mut().unwrap().protocols =
            Some(vec![endpoint(if source == P::Messages { P::Responses } else { P::Messages })]);
        let route = launch_route(agent, &launch).unwrap();
        assert_ne!(route.target, source, "{agent}");
        assert!(route.conversion_required, "{agent}");
    }
}

#[test]
fn model_protocol_013_model_scope_authentication_and_capability_refusals_are_sanitized() {
    use vw_shared::provider::protocols::{ProtocolAuthentication, ProtocolModelScope};
    for (index, expected) in [
        (0, "ModelNotApplicable"),
        (1, "UnsupportedAuthentication"),
        (2, "CapabilityUnavailable"),
        (3, "NoEndpoints"),
        (4, "EndpointNotGranted"),
    ] {
        let mut launch = profile("openai-compatible");
        let mut selected = endpoint(P::Messages);
        match index {
            0 => selected.models = ProtocolModelScope::Models { ids: vec!["other-model".into()] },
            1 => {
                selected.authentication =
                    ProtocolAuthentication::Unsupported { reason: "private-auth-details".into() }
            }
            2 => launch.model_supports_tool_call = false,
            4 => selected.url = "https://other.invalid/private-endpoint".into(),
            _ => {}
        }
        launch.model_api.as_mut().unwrap().protocols =
            Some(if index == 3 { vec![] } else { vec![selected] });
        let error = launch_route("claude", &launch).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("private-auth-details"));
        assert!(!error.contains("private-endpoint"));
        assert!(!error.contains(launch.binding.bearer_token()));
    }
}

#[test]
fn model_protocol_013_unavailable_catalog_is_rejected_before_any_driver_writes_files() {
    for agent in [
        "auggie",
        "claude",
        "codex",
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
        "copilot",
        "cursor",
        "droid",
        "gemini",
        "iflow",
        "kilocode",
        "kimi",
        "kiro",
        "opencode",
        "openclaw",
        "pi",
        "qoder",
        "qwen",
        "trae",
    ] {
        let mut launch = profile("openai-compatible");
        launch.model_api.as_mut().unwrap().protocols = Some(vec![]);
        assert!(configure(agent, &mut launch, "http://127.0.0.1:12345").is_err(), "{agent}");
        assert_eq!(std::fs::read_dir(launch.directory()).unwrap().count(), 0, "{agent}");
    }
}

#[test]
fn model_protocol_023_opencode_uses_proven_http_contract_without_relabeling_sdk() {
    for protocol in [P::Chat, P::Messages, P::Responses, P::Gemini] {
        let mut launch = profile("vendor-sdk");
        launch.model_api.as_mut().unwrap().protocols = Some(vec![endpoint(protocol)]);
        let route = launch_route("opencode", &launch).unwrap();
        assert_eq!(route.source, protocol);
        assert_eq!(route.target, protocol);
        assert_eq!(launch.model_api.as_ref().unwrap().adapter, "vendor-sdk");
    }
    let mut launch = profile("vendor-sdk");
    assert!(launch_route("opencode", &launch).is_err());
    let mut foreign = endpoint(P::Chat);
    foreign.url = "https://foreign.invalid/v1/chat/completions".into();
    launch.model_api.as_mut().unwrap().protocols = Some(vec![foreign]);
    assert!(launch_route("opencode", &launch).is_err());
}
