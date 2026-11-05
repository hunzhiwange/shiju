use crate::tools::*;
use serde_json::json;

#[test]
fn dynamic_tools_contract_namespace_identity_roundtrips() {
    let first = ToolIdentity::new(Some("alpha".into()), "read").unwrap();
    let second = ToolIdentity::new(Some("beta".into()), "read").unwrap();
    assert_ne!(first, second);
    assert_eq!(
        serde_json::from_value::<ToolIdentity>(serde_json::to_value(&first).unwrap()).unwrap(),
        first
    );
}

#[test]
fn dynamic_tools_contract_rejects_empty_identity() {
    for name in ["", " "] {
        assert!(ToolIdentity::new(None, name).is_err());
        assert!(serde_json::from_value::<ToolIdentity>(json!({"name": name})).is_err());
    }
    assert!(ToolIdentity::new(Some("".into()), "read").is_err());
}

fn client_spec() -> ToolSpecDto {
    serde_json::from_value(json!({
        "id":"client-stable", "identity":{"namespace":"alpha","name":"read"},
        "source":"client", "exposure":"deferred", "display_name":"Display",
        "description":"Read data", "input_schema":{"type":"object","properties":{"key":{"type":"string"}}}
    })).unwrap()
}

#[test]
fn dynamic_tools_contract_registration_metadata_roundtrips() {
    for source in [ToolSource::Builtin, ToolSource::Mcp, ToolSource::Client] {
        for exposure in [
            ToolExposure::Direct,
            ToolExposure::Deferred,
            ToolExposure::DirectModelOnly,
            ToolExposure::DeferredModelOnly,
            ToolExposure::CodeModeOnly,
            ToolExposure::Hidden,
        ] {
            let mut spec = client_spec();
            spec.source = source;
            spec.exposure = exposure;
            let dto: ToolSpecDto =
                serde_json::from_value(serde_json::to_value(&spec).unwrap()).unwrap();
            assert_eq!(spec, dto);
            let id = dto.id.clone();
            spec.display_name = "Renamed".into();
            assert_eq!(spec.id, id);
        }
    }
    let spec = client_spec();
    for registration in [
        DynamicToolRegistration::Function(spec.clone()),
        DynamicToolRegistration::Namespace {
            name: "alpha".into(),
            description: "Group".into(),
            tools: vec![spec],
        },
    ] {
        registration.validate().unwrap();
        assert_eq!(
            serde_json::from_value::<DynamicToolRegistration>(
                serde_json::to_value(&registration).unwrap()
            )
            .unwrap(),
            registration
        );
    }
}

#[test]
fn dynamic_tools_contract_rejects_reserved_and_invalid_registration() {
    for name in ["tool_search", "exec", "wait"] {
        let mut spec = client_spec();
        spec.identity.name = name.into();
        assert!(DynamicToolRegistration::Function(spec).validate().is_err());
        let mut spec = client_spec();
        spec.id = name.into();
        assert!(DynamicToolRegistration::Function(spec).validate().is_err());
    }
    for source in [ToolSource::Builtin, ToolSource::Mcp] {
        let mut spec = client_spec();
        spec.source = source;
        assert!(DynamicToolRegistration::Function(spec).validate().is_err());
    }
    let mut spec = client_spec();
    spec.aliases.push("shell".into());
    assert!(DynamicToolRegistration::Function(spec).validate().is_err());
    let mut spec = client_spec();
    spec.id = "".into();
    assert!(DynamicToolRegistration::Function(spec).validate().is_err());
    let mut spec = client_spec();
    spec.input_schema = json!(null);
    assert!(DynamicToolRegistration::Function(spec).validate().is_err());
    for (name, tools) in [
        ("", vec![client_spec()]),
        ("beta", vec![client_spec()]),
        ("alpha", vec![]),
        ("alpha", vec![client_spec(), client_spec()]),
    ] {
        assert!(
            DynamicToolRegistration::Namespace {
                name: name.into(),
                description: String::new(),
                tools
            }
            .validate()
            .is_err()
        );
    }
}

#[test]
fn dynamic_tools_contract_exposure_and_callback_roundtrip() {
    for (exposure, direct, deferred, code) in [
        (ToolExposure::Direct, true, false, true),
        (ToolExposure::Deferred, false, true, true),
        (ToolExposure::DirectModelOnly, true, false, false),
        (ToolExposure::DeferredModelOnly, false, true, false),
        (ToolExposure::CodeModeOnly, false, false, true),
        (ToolExposure::Hidden, false, false, false),
    ] {
        assert_eq!(
            (exposure.is_direct(), exposure.is_deferred(), exposure.is_available_in_code_mode()),
            (direct, deferred, code)
        );
    }
    let response = DynamicToolResponse {
        success: false,
        content: vec![
            DynamicToolContent::Text { text: "result".into() },
            DynamicToolContent::Image { image_url: "data:image/png;base64,AA==".into() },
            DynamicToolContent::Audio { audio_url: "data:audio/wav;base64,AA==".into() },
        ],
    };
    assert_eq!(
        serde_json::from_value::<DynamicToolResponse>(serde_json::to_value(&response).unwrap())
            .unwrap(),
        response
    );
    let request = DynamicToolCallRequest {
        call_id: "call".into(),
        turn_id: "turn".into(),
        tool_id: "stable".into(),
        identity: client_spec().identity,
        arguments: json!({"key":"value"}),
    };
    assert_eq!(
        serde_json::from_value::<DynamicToolCallRequest>(serde_json::to_value(&request).unwrap())
            .unwrap(),
        request
    );
}
