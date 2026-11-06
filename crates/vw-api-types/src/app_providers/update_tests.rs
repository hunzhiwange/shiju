use super::{create::ProviderSecretWrite, update::AppProviderUpdate};
#[test]
fn cc04_update_dto_preserves_missing_fields_and_rejects_null_or_ownership() {
    let input: AppProviderUpdate = serde_json::from_str(r#"{"name":"edited"}"#).unwrap();
    assert!(matches!(input.api_key, ProviderSecretWrite::Keep));
    assert!(input.model.is_none());
    let wire = serde_json::to_value(&input).unwrap();
    assert!(wire.get("model").is_none());
    for value in [
        serde_json::json!({"name":null}),
        serde_json::json!({"model":null}),
        serde_json::json!({"protocol":null}),
        serde_json::json!({"baseUrl":null}),
        serde_json::json!({"apiKey":null}),
        serde_json::json!({"apiKey":"********"}),
        serde_json::json!({"companyId":"other"}),
        serde_json::json!({"addToLive":true}),
        serde_json::json!({"meta":{"notes":"overwrite"}}),
    ] {
        assert!(serde_json::from_value::<AppProviderUpdate>(value).is_err());
    }
    let secret: AppProviderUpdate = serde_json::from_value(
        serde_json::json!({"apiKey":{"action":"replace","value":"fixture-secret"}}),
    )
    .unwrap();
    assert!(!format!("{secret:?}").contains("fixture-secret"));
}
