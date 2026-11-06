use super::app_providers::*;
#[test]
fn cc_apps_provider_launch_isolation_dto_separates_write_secrets_and_launch_identity() {
    let profile: AppProviderWrite = serde_json::from_value(serde_json::json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-key-only","baseUrl":"https://provider.example.test","model":"fixture"})).unwrap();
    assert!(!format!("{profile:?}").contains("fixture-key-only"));
    let request: ProviderLaunchRequest = serde_json::from_value(serde_json::json!({"providerId":"selected","launch":{"appId":"claude","installationId":"observed","cwd":"/work"}})).unwrap();
    let value = serde_json::to_string(&request).unwrap();
    assert!(!value.contains("apiKey"));
    for field in ["tenant", "apiKey", "environment", "command"] {
        let mut value = serde_json::to_value(&request).unwrap();
        value[field] = serde_json::json!("untrusted");
        assert!(serde_json::from_value::<ProviderLaunchRequest>(value).is_err());
    }
}
