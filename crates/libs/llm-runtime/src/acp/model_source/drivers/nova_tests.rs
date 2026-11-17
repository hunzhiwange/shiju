#[test]
fn nova_session_model_uses_native_messages() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "selected", "adapter": "openai-compatible", "url": "https://example.invalid/v1"
    }))
    .unwrap();
    assert_eq!(
        super::source_protocol("nova", &api).unwrap(),
        model_protocols::ModelProtocol::Messages
    );
}
