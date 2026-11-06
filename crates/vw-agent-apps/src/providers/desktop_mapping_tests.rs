use super::desktop_mapping::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::desktop_mapping::*;
fn row(model: &str) -> Value {
    json!({"name":"Fixture","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test","ANTHROPIC_AUTH_TOKEN":"fake-secret","ANTHROPIC_MODEL":model}},"notes":"keep","inFailoverQueue":true})
}
#[test]
fn cc04_desktop_mapping_imports_only_compatible_rows() {
    let direct = import_row(&row("claude-sonnet-5")).unwrap().unwrap();
    assert_eq!(direct["meta"]["claudeDesktopMode"], "direct");
    assert_eq!(direct["notes"], "keep");
    assert_eq!(direct["inFailoverQueue"], false);
    for invalid in [
        json!({"name":"Official","settingsConfig":{"env":{}}}),
        json!({"name":"Bad","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test"}}}),
    ] {
        assert!(import_row(&invalid).unwrap().is_none());
    }
    let mut unsupported = row("claude-sonnet-5");
    unsupported["meta"] = json!({"authBinding":{"accountId":"fake"}});
    assert!(import_row(&unsupported).unwrap().is_none());
}
#[test]
fn cc04_desktop_mapping_rejects_invalid_model_id() {
    for (mode, id, model) in [
        (DesktopMappingMode::Proxy, "gpt-5", "kimi-k2"),
        (DesktopMappingMode::Proxy, "claude-sonnet-", "kimi-k2"),
        (DesktopMappingMode::Proxy, "claude-sonnet-5", "bad\nmodel"),
        (DesktopMappingMode::Direct, "claude-sonnet-5", "kimi-k2"),
        (DesktopMappingMode::Proxy, "claude-sonnet-5", "kimi[1M]"),
    ] {
        let mapping = DesktopMapping {
            mode,
            routes: [(
                id.into(),
                DesktopModelRoute { model: model.into(), label_override: None, supports_1m: false },
            )]
            .into(),
        };
        assert!(validate(&mapping).is_err(), "accepted {mapping:?}");
    }
    assert!(
        validate(&DesktopMapping { mode: DesktopMappingMode::Proxy, routes: Default::default() })
            .is_err()
    );
    assert!(
        validate(&DesktopMapping { mode: DesktopMappingMode::Direct, routes: Default::default() })
            .is_ok()
    );
}
#[test]
fn cc04_desktop_mapping_import_translates_marker_merges_duplicates_and_labels() {
    let mut source = row("unused");
    source["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"] = json!("kimi-k2[1M]");
    source["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"] = json!("kimi-k2");
    source["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL_NAME"] = json!("Kimi");
    let mapped = import_row(&source).unwrap().unwrap();
    assert_eq!(mapped["meta"]["claudeDesktopMode"], "proxy");
    let routes = &mapped["meta"]["claudeDesktopModelRoutes"];
    assert_eq!(routes.as_object().unwrap().len(), 1);
    assert_eq!(routes["claude-sonnet-5"]["model"], "kimi-k2");
    assert_eq!(routes["claude-sonnet-5"]["supports1m"], true);
    assert_eq!(routes["claude-sonnet-5"]["labelOverride"], "Kimi");
    assert_eq!(
        defaults().iter().map(|r| r.route_id.as_str()).collect::<Vec<_>>(),
        ["claude-sonnet-5", "claude-opus-5", "claude-haiku-4-5", "claude-fable-5"]
    );
}
#[test]
fn cc04_desktop_mapping_supports_generic_catalog_and_rejects_unsafe_imports() {
    let generic = json!({"notes":"retain generic notes","name":"Generic","protocol":"anthropic","apiKey":"fake-key","baseUrl":"https://fixture.test","model":"claude-opus-5"});
    let imported = import_row(&generic).unwrap().unwrap();
    assert_eq!(imported["meta"]["claudeDesktopMode"], "direct");
    assert_eq!(imported["notes"], "retain generic notes");
    assert_eq!(imported["claude_desktop"]["config"]["authToken"], "fake-key");
    let mut wrong_type = row("claude-sonnet-5");
    wrong_type["settingsConfig"]["env"]["ANTHROPIC_MODEL"] = json!(7);
    assert!(import_row(&wrong_type).is_err());
    wrong_type = row("claude-sonnet-5");
    wrong_type["meta"] = json!({"apiFormat":7});
    assert!(import_row(&wrong_type).is_err());
    let mut bad = row("bad\nmodel");
    assert!(import_row(&bad).is_err());
    bad = row("claude-sonnet-5");
    bad["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"] = json!("http://remote.test");
    assert!(import_row(&bad).unwrap().is_none());
    bad = row("claude-sonnet-5");
    bad["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("[redacted]");
    assert!(import_row(&bad).unwrap().is_none());
    let mut api_key = row("claude-opus-5");
    let env = api_key["settingsConfig"]["env"].as_object_mut().unwrap();
    env.remove("ANTHROPIC_AUTH_TOKEN");
    env.insert("ANTHROPIC_API_KEY".into(), json!("fake-key"));
    assert_eq!(import_row(&api_key).unwrap().unwrap()["meta"]["claudeDesktopMode"], "proxy");
}
