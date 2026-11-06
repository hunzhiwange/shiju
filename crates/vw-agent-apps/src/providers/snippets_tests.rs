use super::snippets::{extract, normalize};
use serde_json::{Value, json};
use vw_api_types::agent_apps::AppId;
fn parsed(snippet: vw_api_types::app_providers::snippets::ConfigSnippet) -> Value {
    serde_json::from_str(&snippet.content.unwrap()).unwrap()
}
#[test]
fn cc04_snippets_extract_excludes_provider_keys() {
    let settings = json!({"env":{"ANTHROPIC_AUTH_TOKEN":"fixture-key","CLAUDE_CODE_USE_BEDROCK":"1","CLAUDE_CODE_MAX_CONTEXT_TOKENS":"123","OPENROUTER_API_KEY":"fixture-key","DISABLE_TELEMETRY":"1","CLAUDE_CODE_USE_POWERSHELL_TOOL":"1"},"model":"old","apiKeyHelper":"fixture-command","hooks":{"Stop":[]},"theme":"dark"});
    let expected = json!({"env":{"DISABLE_TELEMETRY":"1","CLAUDE_CODE_USE_POWERSHELL_TOOL":"1"},"hooks":{"Stop":[]},"theme":"dark"});
    assert_eq!(parsed(extract(AppId::Claude, &settings).unwrap()), expected);
    assert_eq!(parsed(normalize(AppId::Claude, &settings.to_string()).unwrap()), expected);
    assert_eq!(parsed(extract(AppId::Gemini, &json!({"env":{"GOOGLE_API_KEY":"fixture-key","GEMINI_MODEL":"old","GEMINI_SANDBOX":"true","GROQ_KEY":"fixture-key"}})).unwrap()),json!({"GEMINI_SANDBOX":"true"}));
    let codex = extract(AppId::Codex, &json!({"auth":{"OPENAI_API_KEY":"fixture-key"},"config":"model = 'old'\nweb_search = 'disabled'\nexperimental_bearer_token = 'fixture-key'\nnotify = ['user']\n[model_providers.custom]\nbase_url='https://example.com'\n[mcp_servers.local]\ncommand='fixture'\n[agents]\ndefault_subagent_model='old'\nmax_threads=2\n"})).unwrap();
    let value: Value = toml::from_str(&codex.content.unwrap()).unwrap();
    assert_eq!(value, json!({"notify":["user"],"agents":{"max_threads":2}}));
    assert_eq!(parsed(extract(AppId::OpenCode, &json!({"options":{"apiKey":"fixture-key","baseURL":"https://example.com","timeout":10},"npm":"adapter"})).unwrap()),json!({"options":{"timeout":10},"npm":"adapter"}));
    assert_eq!(parsed(extract(AppId::OpenClaw, &json!({"apiKey":"fixture-key","baseUrl":"https://example.com","api":"openai","models":[]})).unwrap()),json!({"api":"openai","models":[]}));
}
#[cfg(target_os = "macos")]
#[test]
fn cc04_snippets_apply_preserves_live_user_fields() {
    use super::preview::{self, PreviewOwner};
    use vw_api_types::app_providers::preview::ProviderPreviewRequest;
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

#[test]
fn cc04_snippets_normalization_bounds_shapes_and_nested_credentials() {
    assert!(normalize(AppId::Claude, "[]").is_err());
    assert!(normalize(AppId::Claude, "null").is_err());
    assert!(normalize(AppId::Claude, r#"{"env": []}"#).is_err());
    assert!(normalize(AppId::Codex, "[broken").is_err());
    assert!(normalize(AppId::Claude, &"x".repeat(65537)).is_err());
    assert!(normalize(AppId::Kimi, "").is_err());
    assert!(normalize(AppId::Claude, r#"{"theme":"[redacted]"}"#).is_err());
    assert!(normalize(AppId::Claude, "  ").unwrap().content.is_none());
    assert_eq!(parsed(normalize(AppId::Claude,r#"{"plugins":[{"authorization":"fixture-key","password":"fixture-key","keep":true}],"env":{"MAX_THINKING_TOKENS":"42","OTEL_EXPORTER_OTLP_HEADERS":"user"}}"#).unwrap()),json!({"plugins":[{"keep":true}],"env":{"MAX_THINKING_TOKENS":"42","OTEL_EXPORTER_OTLP_HEADERS":"user"}}));
    assert_eq!(parsed(normalize(AppId::Gemini,r#"{"GEMINI_MODEL":"old","GOOGLE_CLOUD_PROJECT":"old","GEMINI_SANDBOX":" true ","blank":" ","nonString":false}"#).unwrap()),json!({"GEMINI_SANDBOX":"true"}));
    let codex=normalize(AppId::Codex,"# fixture-key comment\nweb_search='live'\n[mcp]\nkeep=true\n[mcp.servers.local]\ncommand='fixture'\n[agents]\ndefault_subagent_reasoning_effort='high'\nmax_threads=3\n").unwrap().content.unwrap();
    assert!(!codex.contains("fixture-key"));
    assert_eq!(
        toml::from_str::<Value>(&codex).unwrap(),
        json!({"web_search":"live","mcp":{"keep":true},"agents":{"max_threads":3}})
    );
}
#[test]
fn cc04_snippets_explicit_edits_do_not_restore_an_unchanged_snapshot() {
    use vw_api_types::app_providers::editor_merge::{EditorChoice, EditorEdit, EditorValue};
    let old =
        parsed(normalize(AppId::Claude, r#"{"theme":"old","hooks":{"keep":false}}"#).unwrap());
    let current = br#"{"theme":"current","hooks":{"keep":true},"external":true}"#;
    let edits = vec![
        EditorEdit { path: vec!["theme".into()], value: EditorValue::Set { value: json!("old") } },
        EditorEdit {
            path: vec!["userEdit".into()],
            value: EditorValue::Set { value: json!(true) },
        },
    ];
    let bytes =
        super::editor_merge::merge(&old, current, &edits, EditorChoice::Refuse, true).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        json!({"theme":"current","hooks":{"keep":true},"external":true,"userEdit":true})
    );
}

#[test]
fn cc04_snippets_empty_codex_and_camel_case_credentials_are_safe_to_round_trip() {
    let empty = normalize(AppId::Codex, "# fixture-key comment only\n").unwrap();
    assert!(empty.content.is_none());
    assert_eq!(
        normalize(AppId::Codex, empty.content.as_deref().unwrap_or_default()).unwrap(),
        empty
    );
    let snippet=normalize(AppId::Claude,r#"{"plugin":{"accessToken":"fixture-key","accessKey":"fixture-key","privateKey":"fixture-key","keep":true}}"#).unwrap();
    assert_eq!(parsed(snippet.clone()), json!({"plugin":{"keep":true}}));
    assert_eq!(normalize(AppId::Claude, snippet.content.as_deref().unwrap()).unwrap(), snippet);
}
