use super::universal::{save, summary};
use vw_api_types::app_providers::universal::UniversalProviderWrite;
fn request() -> UniversalProviderWrite {
    serde_json::from_value(serde_json::json!({"config":{"name":"共享","providerType":"custom","apps":{"claude":true,"codex":true,"gemini":true},"baseUrl":"https://example.com/prefix","models":{"claude":{"model":"c","haikuModel":"h","sonnetModel":"s","opusModel":"o"},"codex":{"model":"x","reasoningEffort":"high"},"gemini":{"model":"g"}},"notes":"备注"},"apiKey":{"action":"replace","value":"fixture-secret"}})).unwrap()
}
#[test]
fn cc04_universal_roundtrip_preserves_app_fields() {
    let req = request();
    let saved = save("shared", None, &req).unwrap();
    assert_eq!(saved["config"]["models"]["claude"]["haikuModel"], "h");
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("record.json");
    std::fs::write(&file, serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    stored["future"] = serde_json::json!({"opaque":true});
    stored["meta"] = serde_json::json!({"token":"fixture-token"});
    let mut change = req.clone();
    change.config.name = "Changed".into();
    change.api_key = vw_api_types::app_providers::create::ProviderSecretWrite::Keep;
    let updated = save("shared", Some(&stored), &change).unwrap();
    assert_eq!(updated["config"]["name"], "Changed");
    assert_eq!(updated["config"]["models"], saved["config"]["models"]);
    assert_eq!(updated["future"], stored["future"]);
    assert_eq!(updated["meta"], stored["meta"]);
    assert_eq!(updated["apiKey"], "fixture-secret");
    assert_eq!(summary("shared", &updated).unwrap().config, change.config);
}
#[test]
fn cc04_universal_read_hides_credentials() {
    let req = request();
    let stored = serde_json::json!({"config":req.config,"apiKey":"fixture-secret","meta":{"token":"fixture-token"},"future":{"password":"fixture-password"}});
    let view = summary("shared", &stored).unwrap();
    assert!(view.configured);
    let text = serde_json::to_string(&view).unwrap();
    for secret in ["fixture-secret", "fixture-token", "fixture-password", "apiKey", "meta"] {
        assert!(!text.contains(secret));
    }
    assert!(!format!("{req:?}").contains("fixture-secret"));
}
#[test]
fn cc04_universal_invalid_changes_preserve_saved_record() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let req = request();
    let stored = save("shared", None, &req).unwrap();
    let before = stored.clone();
    let mut change = req.clone();
    change.api_key = ProviderSecretWrite::Keep;
    assert!(save("new", None, &change).is_err());
    for intent in [
        ProviderSecretWrite::Clear,
        ProviderSecretWrite::Replace { value: "[redacted]".into() },
        ProviderSecretWrite::Replace { value: "••••".into() },
        ProviderSecretWrite::Replace { value: "".into() },
    ] {
        change.api_key = intent;
        assert!(save("shared", Some(&stored), &change).is_err());
        assert_eq!(stored, before);
    }
    change.api_key = ProviderSecretWrite::Keep;
    for url in [
        "http://remote.example",
        "https://user:password@example.com",
        "https://example.com?token=fixture",
        "file:///private/file",
    ] {
        change.config.base_url = url.into();
        assert!(save("shared", Some(&stored), &change).is_err());
    }
    change.config = req.config.clone();
    for icon in [
        "/private/icon.png",
        "https://example.com/icon.png",
        "<svg></svg>",
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGAAAABgCAYAAAA=",
    ] {
        change.config.icon = Some(icon.into());
        assert!(save("shared", Some(&stored), &change).is_err());
    }
    change.config.icon=Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==".into());
    assert!(save("shared", Some(&stored), &change).is_ok());
    change.config.icon = Some("openai".into());
    assert!(save("shared", Some(&stored), &change).is_ok());
    let mut corrupt = stored.clone();
    corrupt["apiKey"] = serde_json::json!(false);
    assert!(summary("shared", &corrupt).is_err());
    assert!(save("shared", Some(&corrupt), &req).is_err());
    assert!(save("../invalid", Some(&stored), &req).is_err());
    assert_eq!(stored, before);
}
