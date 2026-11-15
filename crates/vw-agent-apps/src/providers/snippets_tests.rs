use super::snippets::normalize;
use serde_json::Value;
use vw_api_app_identity::{AppId};
fn parsed(snippet: vw_api_app_provider_edit::snippets::ConfigSnippet) -> Value {
    serde_json::from_str(&snippet.content.unwrap()).unwrap()
}
#[cfg(target_os = "macos")]
#[test]
fn cc04_snippets_apply_preserves_live_user_fields() {
    use super::preview::{self, PreviewOwner};
    use vw_api_app_provider_edit::preview::ProviderPreviewRequest;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".claude")).unwrap();
    let file = root.path().join(".claude/settings.json");
    let live = br#"{"hooks":{"keep":true},"theme":"current","env":{"API_TIMEOUT_MS":"10"}}"#;
    std::fs::write(&file, live).unwrap();
    let old = normalize(
        AppId::Claude,
        r#"{"hooks":{"keep":false},"theme":"old","env":{"API_TIMEOUT_MS":"99"}}"#,
    )
    .unwrap();
    let mut settings = parsed(old);
    // A snippet can participate in a draft; only the provider-owned fields are projected.
    settings["env"]["ANTHROPIC_API_KEY"] = json!("fixture-key");
    let request = ProviderPreviewRequest {
        target_host_id: "host".into(),
        directory_override: None,
        settings,
        category: None,
        provider_id: None,
        meta: None,
        request_options: None,
    };
    let projected =
        preview::read(root.path(), AppId::Claude, &request, &PreviewOwner::default()).unwrap();
    assert_eq!(projected.settings["hooks"]["keep"], true);
    assert!(
        !projected
            .changes
            .iter()
            .any(|change| change.path == ["theme"] || change.path == ["env", "API_TIMEOUT_MS"])
    );
    assert_eq!(std::fs::read(file).unwrap(), live);
}
