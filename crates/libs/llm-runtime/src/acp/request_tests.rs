use std::collections::HashMap;
use serde_json::json;
use agent_auth as auth;
use model_types as provider;
use message_types as message;
use super::*;
fn test_model(provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-5-mini",
        "providerID": provider_id,
        "api": {
            "id": "gpt-5-mini",
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": "gpt-5-mini",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn test_provider(key: Option<&str>) -> provider::Info {
    provider::Info {
        id: "github-copilot".to_string(),
        name: "GitHub Copilot".to_string(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

#[test]
fn close_client_after_request_option_defaults_to_false() {
    assert!(!parse_close_client_after_request(&json!({})).expect("missing option should parse"));
}

#[test]
fn close_client_after_request_option_treats_non_object_as_missing() {
    assert!(!parse_close_client_after_request(&json!(null)).expect("null should use the default"));
    assert!(!parse_close_client_after_request(&json!([])).expect("array should use the default"));
}

#[test]
fn close_client_after_request_option_accepts_boolean_only() {
    assert!(
        parse_close_client_after_request(&json!({"acp_close_client_after_request": true}))
            .expect("boolean option should parse")
    );
    assert!(
        parse_close_client_after_request(&json!({"acp_close_client_after_request": "true"}))
            .is_err()
    );
}

#[test]
fn close_client_after_request_option_accepts_explicit_false() {
    assert!(
        !parse_close_client_after_request(&json!({"acp_close_client_after_request": false}))
            .expect("false should remain disabled")
    );
}

#[test]
fn close_client_after_request_option_reports_non_boolean_error() {
    let error = parse_close_client_after_request(&json!({
        "acp_close_client_after_request": 1
    }))
    .expect_err("numeric option should be rejected");

    match error {
        Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[test]
fn close_client_after_request_option_rejects_explicit_null() {
    let error = parse_close_client_after_request(&json!({
        "acp_close_client_after_request": null
    }))
    .expect_err("explicit null must not silently use the default");

    match error {
        Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[test]
fn agent_model_inheritance_validates_credentials_and_model_snapshot() {
    let mut model = test_model("github-copilot");
    let info = test_provider(Some("fixture-provider-key"));
    assert!(super::validate_session_model(&model, &info, None).is_ok());
    assert!(
        super::validate_session_model(&model, &test_provider(Some(auth::OAUTH_DUMMY_KEY)), None)
            .is_err()
    );
    let oauth = auth::Info::Oauth(auth::OauthInfo {
        access: String::new(),
        refresh: "fixture-refresh".into(),
        expires: 0,
        account_id: None,
        enterprise_url: None,
    });
    assert!(super::validate_session_model(&model, &test_provider(None), Some(&oauth)).is_ok());
    model.api.adapter = "acp".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
    model.api.adapter = "openai-compatible".into();
    model.status = "disabled".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
    model.status = "active".into();
    model.provider_id = "different-provider".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
}
