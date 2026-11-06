use super::preview::{self, PreviewOwner};
fn read(
    home: &std::path::Path,
    app: AppId,
    request: &ProviderPreviewRequest,
) -> Result<ProviderPreview, &'static str> {
    preview::read(home, app, request, &PreviewOwner::default())
}

use serde_json::json;
use vw_api_types::{agent_apps::AppId, app_providers::preview::*};
fn request(settings: serde_json::Value) -> ProviderPreviewRequest {
    ProviderPreviewRequest {
        request_options: None,
        target_host_id: "host".into(),
        directory_override: None,
        settings,
        category: None,
        provider_id: None,
        meta: None,
    }
}
#[test]
fn cc04_preview_never_writes_files() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".claude")).unwrap();
    let path = root.path().join(".claude/settings.json");
    let before = br#"{"env":{"ANTHROPIC_API_KEY":"old-secret","API_TIMEOUT_MS":"10"},"hooks":{"keep":true}}"#;
    std::fs::write(&path, before).unwrap();
    let value=read(root.path(),AppId::Claude,&request(json!({"env":{"ANTHROPIC_API_KEY":"new-secret","ANTHROPIC_MODEL":"new-model","API_TIMEOUT_MS":"99"},"hooks":{"keep":false}}))).unwrap();
    assert_eq!(value.settings["hooks"]["keep"], true);
    assert!(value.changes.iter().any(|c| c.path == ["env", "ANTHROPIC_MODEL"]));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    let response = serde_json::to_string(&value).unwrap();
    for secret in ["old-secret", "new-secret", "new-model", "99"] {
        assert!(!response.contains(secret));
    }
}
#[test]
fn cc04_preview_reports_inactive_fields_and_parse_location() {
    let root = tempfile::tempdir().unwrap();
    let req = request(json!({"env":{"API_TIMEOUT_MS":"99"},"hooks":{"keep":false}}));
    let value = read(root.path(), AppId::Claude, &req).unwrap();
    assert!(value.inactive.contains(&vec!["hooks".into()]));
    assert!(value.inactive.contains(&vec!["env".into(), "API_TIMEOUT_MS".into()]));
    std::fs::create_dir(root.path().join(".claude")).unwrap();
    std::fs::write(root.path().join(".claude/settings.json"), "{\n  \"env\": invalid-secret\n}")
        .unwrap();
    let value = read(root.path(), AppId::Claude, &req).unwrap();
    let error = value.parse_error.unwrap();
    assert_eq!(error.source, "live.settings");
    assert_eq!(error.line, 2);
    assert!(error.column > 0);
}

type Snapshot =
    std::collections::BTreeMap<std::path::PathBuf, (Vec<u8>, u32, std::time::SystemTime)>;
fn snapshot(root: &std::path::Path) -> Snapshot {
    use std::os::unix::fs::PermissionsExt;
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut Snapshot) {
        for entry in std::fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            let data = if meta.is_file() {
                std::fs::read(&path).unwrap()
            } else if meta.file_type().is_symlink() {
                std::fs::read_link(&path).unwrap().as_os_str().as_encoded_bytes().to_vec()
            } else {
                vec![]
            };
            out.insert(
                path.strip_prefix(root).unwrap().to_path_buf(),
                (data, meta.permissions().mode(), meta.modified().unwrap()),
            );
            if meta.is_dir() {
                walk(root, &path, out);
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn fixture(root: &std::path::Path, relative: &str, text: &str) {
    let p = root.join(relative);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}
#[test]
fn cc04_preview_claude_preserves_user_exclusive_values_and_converts_bedrock() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        ".claude/settings.json",
        r#"{"env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"1","ENABLE_TOOL_SEARCH":"user","CLAUDE_CODE_MAX_CONTEXT_TOKENS":"262144","ANTHROPIC_AUTH_TOKEN":"old","CLAUDE_CODE_USE_POWERSHELL_TOOL":"1"}}"#,
    );
    let owner = PreviewOwner {
        settings: Some(
            json!({"env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"1","ENABLE_TOOL_SEARCH":"owner"}}),
        ),
        ..Default::default()
    };
    let req = request(
        json!({"apiKey":"bedrock-secret","env":{"CLAUDE_CODE_USE_BEDROCK":"1","AWS_REGION":"fixture-region"}}),
    );
    let before = snapshot(root.path());
    let view = preview::read(root.path(), AppId::Claude, &req, &owner).unwrap();
    let env = &view.settings["env"];
    assert!(env.get("CLAUDE_CODE_DISABLE_ARTIFACT").is_none());
    assert!(env.get("CLAUDE_CODE_MAX_CONTEXT_TOKENS").is_none());
    assert!(env.get("ANTHROPIC_AUTH_TOKEN").is_none());
    assert_eq!(env["ENABLE_TOOL_SEARCH"], "[redacted]");
    assert_eq!(env["AWS_BEARER_TOKEN_BEDROCK"], "[redacted]");
    assert!(view.settings.get("apiKey").is_none());
    assert_eq!(env["CLAUDE_CODE_USE_POWERSHELL_TOOL"], "[redacted]");
    assert_eq!(snapshot(root.path()), before);
}
#[test]
fn cc04_preview_gemini_projects_both_files_and_official_auth_only() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        ".gemini/.env",
        "# user\nGEMINI_API_KEY=old-secret\nGOOGLE_CLOUD_PROJECT=old\nGEMINI_SANDBOX=docker\n",
    );
    fixture(
        root.path(),
        ".gemini/settings.json",
        r#"{"security":{"auth":{"selectedType":"oauth-personal","useExternal":true}},"model":{"name":"old","compressionThreshold":0.5},"ui":{"theme":"dark"}}"#,
    );
    let mut req = request(
        json!({"env":{"GEMINI_API_KEY":"draft-secret","GOOGLE_GEMINI_BASE_URL":"https://fixture.test","GEMINI_SANDBOX":"none"},"config":{"security":{"auth":{"selectedType":"oauth-personal"}},"model":{"name":"new","compressionThreshold":0.8},"ui":{"theme":"light"}}}),
    );
    let before = snapshot(root.path());
    let v = read(root.path(), AppId::Gemini, &req).unwrap();
    assert!(v.settings["env"].get("GOOGLE_CLOUD_PROJECT").is_none());
    assert_eq!(v.settings["config"]["model"]["compressionThreshold"], 0.5);
    assert_eq!(v.settings["config"]["security"]["auth"]["useExternal"], true);
    assert!(v.inactive.contains(&vec!["env".into(), "GEMINI_SANDBOX".into()]));
    assert!(v.inactive.contains(&vec!["config".into(), "ui".into()]));
    assert!(v.changes.iter().any(|c| c.path == ["config", "security", "auth", "selectedType"]));
    req.category = Some("official".into());
    req.settings = json!({"env":{},"config":{}});
    let v = read(root.path(), AppId::Gemini, &req).unwrap();
    assert!(!v.changes.iter().any(|c| c.path == ["config", "security", "auth", "selectedType"]));
    assert!(v.settings["config"]["model"].get("name").is_none());
    assert_eq!(snapshot(root.path()), before);
}
#[test]
fn cc04_preview_codex_normalizes_routes_and_reports_global_inactive_fields() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        ".codex/config.toml",
        "# untouched\nmodel_provider='custom'\nmodel='old'\n[model_providers.custom]\nbase_url='https://old.test'\nexperimental_bearer_token='live-secret'\n[projects.fixture]\ntrust_level='trusted'\n[agents]\ndefault_subagent_model='old'\nmax_threads=3\n",
    );
    fixture(root.path(), ".codex/auth.json", r#"{}"#);
    let req = request(
        json!({"auth":{},"config":"model_provider='vendor'\nmodel='new'\n[model_providers.vendor]\nbase_url='https://fixture.test'\nrequires_openai_auth=true\n[projects.fixture]\ntrust_level='untrusted'\n"}),
    );
    let before = snapshot(root.path());
    let v = read(root.path(), AppId::Codex, &req).unwrap();
    assert!(v.parse_error.is_none());
    assert!(v.unavailable.is_none());
    assert!(v.settings["config"]["model_providers"].get("custom").is_some());
    assert!(v.settings["config"]["model_providers"].get("vendor").is_none());
    assert!(
        v.settings["config"]["model_providers"]["custom"]
            .get("experimental_bearer_token")
            .is_none()
    );
    assert_eq!(v.settings["config"]["model_providers"]["custom"]["requires_openai_auth"], false);
    assert_eq!(v.settings["config"]["agents"]["max_threads"], 3);
    assert!(v.settings["config"]["agents"].get("default_subagent_model").is_none());
    assert!(v.inactive.contains(&vec!["config".into(), "projects".into(), "fixture".into()]));
    assert!(!serde_json::to_string(&v).unwrap().contains("secret"));
    assert_eq!(snapshot(root.path()), before);
}
#[test]
fn cc04_preview_codex_builtins_official_and_parse_locations_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    for config in [
        "model_provider='ollama'\nmodel='local'\n",
        "model_provider='amazon-bedrock'\n[model_providers.amazon-bedrock]\nregion='fixture'\n",
        "model='m'\n",
    ] {
        let v =
            read(root.path(), AppId::Codex, &request(json!({"auth":{},"config":config}))).unwrap();
        assert!(v.unavailable.is_none());
        assert!(v.parse_error.is_none());
    }
    let mut req = request(json!({"auth":{},"config":"model='m'"}));
    req.category = Some("official".into());
    assert_eq!(
        read(root.path(), AppId::Codex, &req).unwrap().unavailable,
        Some(PreviewUnavailable::OfficialLogin)
    );
    req.category = None;
    req.settings["config"] = json!("model =\n secret");
    let v = read(root.path(), AppId::Codex, &req).unwrap();
    assert_eq!(v.parse_error.unwrap().source, "draft.config");
    fixture(
        root.path(),
        ".codex/config.toml",
        "profile='conflict'\n[profiles.conflict]\nmodel_provider='other'\n",
    );
    req.settings["config"] = json!("model_provider='ollama'");
    assert_eq!(
        read(root.path(), AppId::Codex, &req).unwrap().unavailable,
        Some(PreviewUnavailable::Ownership)
    );
}
fn grok_row() -> serde_json::Value {
    json!({"config":"[models]\ndefault='vendor'\n[model.vendor]\nmodel='new'\nname='fixture'\nbase_url='https://fixture.test'\napi_key='draft-secret'\napi_backend='responses'\ncontext_window=1000\nextra_headers={Vendor='header-secret'}\n[ui]\ncompact=false\n"})
}
#[test]
fn cc04_preview_grok_retires_owned_tables_and_follows_session_summary() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        ".grok/config.toml",
        "# user\n[models]\ndefault='builtin'\nsession_summary='old'\n[model.old]\napi_key='old-secret'\n[model.user]\napi_key='user-secret'\n[ui]\ncompact=true\n",
    );
    let owner = PreviewOwner {
        settings: Some(
            json!({"config":"[models]\ndefault='old'\n[model.old]\napi_key='old-secret'\n"}),
        ),
        ..Default::default()
    };
    let before = snapshot(root.path());
    let req = request(grok_row());
    let v = preview::read(root.path(), AppId::GrokBuild, &req, &owner).unwrap();
    assert!(v.settings["config"]["model"].get("old").is_none());
    assert!(v.settings["config"]["model"].get("user").is_some());
    assert!(v.settings["config"]["model"]["vendor"].get("extra_headers").is_some());
    assert_eq!(v.settings["config"]["ui"]["compact"], true);
    assert!(v.inactive.contains(&vec!["config".into(), "ui".into(), "compact".into()]));
    let mut official = req.clone();
    official.category = Some("official".into());
    let v = preview::read(root.path(), AppId::GrokBuild, &official, &owner).unwrap();
    assert!(v.settings["config"]["models"].get("default").is_none());
    assert!(v.settings["config"]["models"].get("session_summary").is_none());
    assert_eq!(snapshot(root.path()), before);
}
#[test]
fn cc04_preview_rejects_symlinks_and_unsupported_targets_without_provisioning() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    fixture(external.path(), "settings.json", r#"{"env":{"ANTHROPIC_API_KEY":"outside-secret"}}"#);
    std::os::unix::fs::symlink(external.path(), root.path().join(".claude")).unwrap();
    let before = snapshot(root.path());
    let outside = snapshot(external.path());
    assert!(read(root.path(), AppId::Claude, &request(json!({}))).is_err());
    assert_eq!(
        read(root.path(), AppId::OpenCode, &request(json!({}))).unwrap().unavailable,
        Some(PreviewUnavailable::UnsupportedApplication)
    );
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(snapshot(external.path()), outside);
}
#[test]
fn cc04_preview_opening_existing_transactions_does_not_create_or_recover() {
    use super::live::commit::{ExecutionHostId, ProviderTransactions};
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let before = snapshot(root.path());
    assert!(
        ProviderTransactions::open_existing(root.path(), ExecutionHostId::new("host").unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(snapshot(root.path()), before);
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    assert!(tx.state(AppId::Claude).unwrap().pointer.is_none());
    let before = snapshot(root.path());
    let observer =
        ProviderTransactions::open_existing(root.path(), ExecutionHostId::new("host").unwrap())
            .unwrap()
            .unwrap();
    assert!(observer.pending(AppId::Claude).unwrap().is_none());
    assert!(observer.state(AppId::Claude).unwrap().pointer.is_none());
    assert_eq!(snapshot(root.path()), before);
}
#[test]
fn cc04_preview_grok_invalid_draft_and_codex_catalog_are_explicitly_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let mut row = grok_row();
    row["config"] =
        json!("[models]\ndefault='missing'\n[model.a]\nname='one'\n[model.b]\nname='two'\n");
    let view = read(root.path(), AppId::GrokBuild, &request(row)).unwrap();
    assert_eq!(view.unavailable, Some(PreviewUnavailable::InvalidDraft));
    assert!(view.settings["config"]["model"].get("a").is_some());
    assert!(view.changes.is_empty());
    let view = read(
        root.path(),
        AppId::Codex,
        &request(json!({"auth":{},"config":"model='m'","modelCatalog":{"models":[{"model":"m"}]}})),
    )
    .unwrap();
    assert_eq!(view.unavailable, Some(PreviewUnavailable::ModelCatalog));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[test]
fn cc04_preview_credentials_are_compared_before_redaction_and_metadata_is_explicit() {
    let root = tempfile::tempdir().unwrap();
    fixture(
        root.path(),
        ".codex/config.toml",
        "model_provider='custom'\n[model_providers.custom]\nname='Custom'\nbase_url='https://fixture.test'\nrequires_openai_auth=false\nexperimental_bearer_token='same-secret'\n",
    );
    let mut req = request(
        json!({"auth":{"OPENAI_API_KEY":"same-secret"},"config":"model_provider='custom'\n[model_providers.custom]\nname='Custom'\nbase_url='https://fixture.test'\nrequires_openai_auth=false\n"}),
    );
    let v = read(root.path(), AppId::Codex, &req).unwrap();
    assert!(!v.changes.iter().any(|c|c.path==["config","model_providers","custom","experimental_bearer_token"]));
    req.settings["auth"]["OPENAI_API_KEY"] = json!("rotated-secret");
    let v = read(root.path(), AppId::Codex, &req).unwrap();
    assert!(v.changes.iter().any(|c|c.path==["config","model_providers","custom","experimental_bearer_token"]));
    assert!(!serde_json::to_string(&v).unwrap().contains("secret"));
    req.meta = Some(json!({"providerType":"github_copilot"}));
    assert_eq!(
        read(root.path(), AppId::Codex, &req).unwrap().unavailable,
        Some(PreviewUnavailable::ManagedProvider)
    );
    req.meta = Some(json!({}));
    assert!(read(root.path(), AppId::Codex, &req).unwrap().unavailable.is_none());
    fixture(
        root.path(),
        ".codex/auth.json",
        r#"{"tokens":{"access_token":"native-login-secret"}}"#,
    );
    assert_eq!(
        read(root.path(), AppId::Codex, &req).unwrap().unavailable,
        Some(PreviewUnavailable::OfficialLogin)
    );
    let mut req = request(json!({"apiKey":12345,"env":{"ANTHROPIC_API_KEY":false}}));
    req.meta = None;
    let v = read(root.path(), AppId::Claude, &req).unwrap();
    assert_eq!(v.settings["apiKey"], "[redacted]");
    assert_eq!(v.settings["env"]["ANTHROPIC_API_KEY"], "[redacted]");
}
#[test]
fn cc04_preview_existing_official_categories_follow_source_rules() {
    use super::preview::editor_category;
    assert_eq!(
        editor_category(AppId::Gemini, "fixture", &json!({"name":"Google Legacy"}), None)
            .as_deref(),
        Some("official")
    );
    assert_eq!(
        editor_category(
            AppId::Gemini,
            "fixture",
            &json!({"name":"Google Legacy","meta":{"partnerPromotionKey":"packycode"}}),
            None
        ),
        None
    );
    assert_eq!(
        editor_category(
            AppId::Codex,
            "codex-official",
            &json!({"settingsConfig":{"auth":{},"config":""}}),
            None
        )
        .as_deref(),
        Some("official")
    );
    assert_eq!(
        editor_category(
            AppId::Codex,
            "codex-official",
            &json!({"settingsConfig":{"auth":{},"config":"model_provider='custom'"}}),
            None
        ),
        None
    );
}
