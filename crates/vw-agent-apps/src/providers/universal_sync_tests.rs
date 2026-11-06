use super::universal_sync::*;
use serde_json::json;
use vw_api_types::{agent_apps::AppId, app_providers::universal_sync::*};
fn parent() -> serde_json::Value {
    json!({"config":{"name":"Shared","providerType":"custom","apps":{"claude":true,"codex":true,"gemini":true},"baseUrl":"https://example.com","models":{"claude":{"model":"claude-main","haikuModel":"haiku"},"codex":{"model":"codex-main","reasoningEffort":"low"},"gemini":{"model":"gemini-main"}}},"apiKey":"fixture-key"})
}
#[test]
fn cc04_universal_sync_updates_only_selected_apps() {
    let root = tempfile::tempdir().unwrap();
    let request = UniversalSyncRequest { apps: vec![AppId::Claude, AppId::Codex] };
    validate_request(&request).unwrap();
    for &app in &request.apps {
        let id = child_id(app, "shared").unwrap();
        let value = project("shared", &parent(), app, None).unwrap();
        let path = root.path().join(id);
        std::fs::write(&path, value.to_string()).unwrap();
        let mut saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        saved["meta"] = json!({"commonConfigEnabled":false,"usageScript":{"token":"local-secret"}});
        saved["sortIndex"] = json!(42);
        saved["customEndpoints"] = json!(["https://extra.example"]);
        saved["settingsConfig"]["local"] = json!({"preserve":true});
        if app == AppId::Codex {
            let raw = saved["settingsConfig"]["config"].as_str().unwrap();
            saved["settingsConfig"]["config"] = json!(format!(
                "sandbox_mode = \"read-only\"\n{raw}\n[model_providers.other]\nname = \"keep\"\n"
            ));
        }
        let mut updated_parent = parent();
        updated_parent["config"]["name"] = json!("Changed");
        let updated = project("shared", &updated_parent, app, Some(&saved)).unwrap();
        assert_eq!(updated["name"], "Changed");
        for key in ["meta", "sortIndex", "customEndpoints"] {
            assert_eq!(updated[key], saved[key]);
        }
        assert_eq!(updated["settingsConfig"]["local"], saved["settingsConfig"]["local"]);
        assert_eq!(updated["apiKey"], "fixture-key");
        if app == AppId::Claude {
            assert_eq!(updated["model"], "claude-main");
            assert_eq!(updated["settingsConfig"]["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "haiku");
        } else {
            assert_eq!(updated["baseUrl"], "https://example.com/v1");
            let doc: toml::Table =
                updated["settingsConfig"]["config"].as_str().unwrap().parse().unwrap();
            assert_eq!(doc["model_reasoning_effort"].as_str(), Some("low"));
            assert_eq!(doc["sandbox_mode"].as_str(), Some("read-only"));
            assert_eq!(doc["model_providers"]["other"]["name"].as_str(), Some("keep"));
            assert_eq!(
                doc["model_providers"]["custom"]["base_url"].as_str(),
                Some("https://example.com/v1")
            );
        }
    }
    assert!(!root.path().join("universal-gemini-shared").exists());
    let mut custom = parent();
    custom["config"]["baseUrl"] = json!("https://example.com/prefix/");
    assert_eq!(
        project("shared", &custom, AppId::Codex, None).unwrap()["baseUrl"],
        "https://example.com/prefix"
    );
    assert_eq!(project("shared", &parent(), AppId::Gemini, None).unwrap()["model"], "gemini-main");
}
#[test]
fn cc04_universal_sync_failure_has_explicit_result() {
    assert_eq!(child_id(AppId::Kimi, "shared").unwrap_err(), UniversalSyncError::UnsupportedApp);
    let mut disabled = parent();
    disabled["config"]["apps"]["claude"] = json!(false);
    assert_eq!(
        project("shared", &disabled, AppId::Claude, None).unwrap_err(),
        UniversalSyncError::AppDisabled
    );
    let existing = json!({"name":"unrelated","apiKey":"local-key"});
    assert_eq!(
        project("shared", &parent(), AppId::Claude, Some(&existing)).unwrap_err(),
        UniversalSyncError::TargetConflict
    );
    assert_eq!(existing["apiKey"], "local-key");
    for apps in [vec![], vec![AppId::Claude, AppId::Claude]] {
        assert!(validate_request(&UniversalSyncRequest { apps }).is_err());
    }
    assert_eq!(
        child_id(AppId::Claude, &"a".repeat(128)).unwrap_err(),
        UniversalSyncError::InvalidTargetIdentity
    );
}
