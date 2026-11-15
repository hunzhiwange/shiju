use serde_json::json;

#[test]
fn acp_model_source_contract_session_controls_default_and_preserve_source() {
    use crate::acp_session::AcpSessionOptions;
    let legacy: AcpSessionOptions =
        serde_json::from_value(json!({"connected": false, "options": []})).unwrap();
    assert_eq!(serde_json::to_value(legacy).unwrap()["acp_model_source"], "session_model");
    let local: AcpSessionOptions = serde_json::from_value(json!({
        "connected": true, "options": [], "acp_model_source": "cli_local"
    }))
    .unwrap();
    assert_eq!(serde_json::to_value(local).unwrap()["acp_model_source"], "cli_local");
}

#[test]
fn acp_model_source_contract_saved_selection_defaults_without_copying_credentials() {
    use crate::acp_model_source::{AcpModelSource, AcpSessionModelSelection};
    let selection: AcpSessionModelSelection = serde_json::from_value(json!({
        "provider_id": "provider-a", "model": "provider-a/org/model"
    }))
    .unwrap();
    assert_eq!(selection.source, AcpModelSource::SessionModel);
    assert_eq!(
        serde_json::to_value(selection).unwrap(),
        json!({
            "source": "session_model", "provider_id": "provider-a", "model": "provider-a/org/model"
        })
    );
    assert!(
        serde_json::from_value::<AcpSessionModelSelection>(json!({"api_key":"secret"})).is_err()
    );
}
