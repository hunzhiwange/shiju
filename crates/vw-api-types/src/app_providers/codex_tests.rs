use super::preview::*;
#[test]
fn cc04_codex_readonly_dto_rejects_publish_fields_and_conceals_input() {
    let input = serde_json::json!({"targetHostId":"host","settings":{"auth":{"OPENAI_API_KEY":"fake-route-key"},"config":"model_provider='vendor'"}});
    let request: ProviderPreviewRequest = serde_json::from_value(input.clone()).unwrap();
    assert!(!format!("{request:?}").contains("fake-route-key"));
    for field in ["publish", "authFile", "loginOnDisk", "tenant", "file"] {
        let mut invalid = input.clone();
        invalid[field] = serde_json::json!(true);
        assert!(serde_json::from_value::<ProviderPreviewRequest>(invalid).is_err());
    }
    let response = ProviderPreview {
        settings: serde_json::Value::Null,
        changes: vec![],
        inactive: vec![],
        unavailable: Some(PreviewUnavailable::Ownership),
        parse_error: None,
    };
    assert_eq!(
        serde_json::from_value::<ProviderPreview>(serde_json::to_value(&response).unwrap())
            .unwrap(),
        response
    );
}
