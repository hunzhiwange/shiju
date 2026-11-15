use super::preview::*;
#[test]
fn cc04_preview_request_is_write_only_and_rejects_arbitrary_paths() {
    let input = serde_json::json!({"targetHostId":"host","directoryOverride":null,"settings":{"key":"secret"},"category":null});
    let request: ProviderPreviewRequest = serde_json::from_value(input.clone()).unwrap();
    assert!(!format!("{request:?}").contains("secret"));
    let mut invalid = input;
    invalid["file"] = serde_json::json!("/etc/passwd");
    assert!(serde_json::from_value::<ProviderPreviewRequest>(invalid).is_err());
    let response = ProviderPreview {
        settings: serde_json::json!({"key":"[redacted]"}),
        changes: vec![],
        inactive: vec![],
        unavailable: Some(PreviewUnavailable::OfficialLogin),
        parse_error: None,
    };
    assert_eq!(
        serde_json::from_value::<ProviderPreview>(serde_json::to_value(&response).unwrap())
            .unwrap(),
        response
    );
}
