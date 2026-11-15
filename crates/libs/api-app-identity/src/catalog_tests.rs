use super::{AppCatalogResponse, AppId, AppRuntimeKind};
use serde_json::json;

#[test]
fn cc_apps_catalog_roundtrip_all_public_ids() {
    for id in [
        "claude",
        "claude-desktop",
        "codex",
        "gemini",
        "grokbuild",
        "opencode",
        "openclaw",
        "hermes",
        "pi",
        "mcode",
        "kimi",
        "dsh",
        "agy",
        "omp",
    ] {
        let parsed: AppId = serde_json::from_value(json!(id)).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), json!(id));
    }
}
#[test]
fn cc_apps_catalog_roundtrip_rejects_unknown_and_internal_ids() {
    for id in ["grok", "unknown", "Claude", "claude_desktop", "claudeDesktop", "", " codex"] {
        assert!(serde_json::from_value::<AppId>(json!(id)).is_err(), "{id}");
    }
}
#[test]
fn cc_apps_catalog_roundtrip_distinct_runtime_kinds() {
    let mut parsed = Vec::new();
    for kind in ["native_cli", "acp_adapter", "review_runtime", "desktop_app", "editor_integration"]
    {
        let value: AppRuntimeKind = serde_json::from_value(json!(kind)).unwrap();
        assert!(!parsed.contains(&value));
        parsed.push(value);
        assert_eq!(serde_json::to_value(value).unwrap(), json!(kind));
    }
    assert!(serde_json::from_value::<AppRuntimeKind>(json!("cli")).is_err());
}
#[test]
fn cc_apps_catalog_roundtrip_response_preserves_runtime_identity() {
    let wire = json!({"apps": [{"id": "grokbuild", "name": "Grok Build", "cliCommand": "grok", "runtimeKinds": ["native_cli", "acp_adapter"]}, {"id": "claude-desktop", "name": "Claude Desktop", "cliCommand": null, "runtimeKinds": ["desktop_app"]}]});
    let response: AppCatalogResponse = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), wire);
    let mut invalid = wire;
    invalid["apps"][0]["id"] = json!("grok");
    assert!(serde_json::from_value::<AppCatalogResponse>(invalid).is_err());
    let empty: AppCatalogResponse = serde_json::from_value(json!({"apps": []})).unwrap();
    assert!(empty.apps.is_empty());
}
