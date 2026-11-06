use crate::{
    agent_apps::AppId,
    app_providers::{ProviderProtocol, create::*},
};
fn request() -> AppProviderCreate {
    AppProviderCreate {
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: ProviderSecretWrite::Replace { value: "fixture-private-key".into() },
        base_url: "https://fixture.example.test".into(),
        model: "fixture-model".into(),
        add_to_live: false,
    }
}
#[test]
fn cc04_create_dto_requires_explicit_live_and_secret_intent_and_redacts_debug() {
    let input = request();
    let mut json = serde_json::to_value(&input).unwrap();
    assert_eq!(json["addToLive"], false);
    assert_eq!(json["apiKey"]["action"], "replace");
    assert!(!format!("{input:?}").contains("fixture-private-key"));
    assert!(serde_json::from_value::<AppProviderCreate>(json.clone()).is_ok());
    json.as_object_mut().unwrap().remove("addToLive");
    assert!(serde_json::from_value::<AppProviderCreate>(json).is_err());
    let mut json = serde_json::to_value(&input).unwrap();
    json["apiKey"] = serde_json::json!("implicit-string");
    assert!(serde_json::from_value::<AppProviderCreate>(json).is_err());
    let mut json = serde_json::to_value(&input).unwrap();
    json["targetHostId"] = serde_json::json!("arbitrary-host");
    assert!(serde_json::from_value::<AppProviderCreate>(json).is_err());
    let _ = AppId::Claude;
}
