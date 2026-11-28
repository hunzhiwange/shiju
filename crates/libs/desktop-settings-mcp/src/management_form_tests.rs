use super::*;
use vw_api_app_auth::AuthService;
use vw_api_mcp_management::McpCredentialRef;
use vw_api_transport::TenantScope;
fn context() -> crate::management_context::Context {
    crate::management_context::Context {
        gateway: "http://localhost:1".into(),
        tenant: TenantScope::new("10000", "100000001").unwrap(),
        host: "host".into(),
    }
}
fn draft() -> Draft {
    let mut draft = Draft::new(context());
    draft.id = "demo".into();
    draft.command = "fixture-command".into();
    draft.args = r#"["a,b","空 格",""]"#.into();
    draft.values = serde_json::to_string(&BTreeMap::from([(
        "TOKEN",
        McpConfigValue::Credential {
            reference: McpCredentialRef {
                service: AuthService::CodexOauth,
                account_id: "fixture-account".into(),
            },
        },
    )]))
    .unwrap();
    draft
}
#[test]
fn cc_mcp_t45_graphical_json_roundtrip_preserves_arguments_and_credential_references() {
    let mut draft = draft();
    let connection = draft.connection().unwrap();
    draft.toggle_json(true).unwrap();
    assert!(draft.connection().unwrap() == connection);
    draft.toggle_json(false).unwrap();
    assert!(draft.connection().unwrap() == connection);
    assert_eq!(draft.args, r#"["a,b","空 格",""]"#);
    for transport in ["streamable_http", "sse"] {
        draft.transport = transport.into();
        draft.url = "http://127.0.0.1/mcp".into();
        let connection = draft.connection().unwrap();
        draft.toggle_json(true).unwrap();
        draft.toggle_json(false).unwrap();
        assert!(draft.connection().unwrap() == connection);
    }
}
#[test]
fn cc_mcp_t45_unknown_masks_are_rejected_in_both_editors() {
    let mut draft = draft();
    draft.values = r#"{"TOKEN":{"kind":"literal","value":"••••••••"}}"#.into();
    assert!(draft.connection().is_err(), "unknown mask must not become a saved literal");
    draft.json = true;
    draft.editor = text_editor::Content::with_text(
        r#"{"transport":"sse","url":"http://localhost/mcp","headers":{"Authorization":{"kind":"credential","reference":{"service":"codex_oauth","accountId":"••••••••"}}}}"#,
    );
    assert!(draft.connection().is_err());
}
#[test]
fn cc_mcp_t45_json_errors_locate_without_echoing_payload_and_preserve_draft() {
    let mut draft = draft();
    draft.json = true;
    draft.editor = text_editor::Content::with_text("{\n\"private-secret\": }");
    let error = draft.toggle_json(false).unwrap_err();
    assert!(error.contains("第 2 行"));
    assert!(!error.contains("private-secret"));
    assert!(draft.json);
    assert!(draft.editor.text().contains("private-secret"));
    draft.saving = true;
    draft.saving = false;
    draft.error = Some("修订冲突".into());
    assert!(!draft.saving);
    assert_eq!(draft.error.as_deref(), Some("修订冲突"));
    assert!(draft.editor.text().contains("private-secret"));
}

#[test]
fn global_form_keeps_runtime_options_and_exact_command_arguments() {
    let original = serde_json::json!({
        "name": "demo",
        "command": "fixture-command",
        "args": ["a,b", "空 格", ""],
        "env": [{"name": "MODE", "value": "test"}],
        "enabled": false,
        "required": true,
        "timeout": 12,
        "allowedTools": ["read"],
        "_meta": {"custom": true},
    });
    let mut page = crate::state::State::default();
    page.editor = text_editor::Content::with_text(&serde_json::json!([&original]).to_string());
    let mut draft = Draft::new(context());
    global::open(&mut draft, &page, Some(0)).unwrap();
    draft.edit(FormMessage::Field("command", "updated-command".into()));

    let global = &draft.global;
    let updated = global::server(&draft, global).unwrap();
    assert_eq!(global.original.as_ref(), Some(&original));
    assert_eq!(updated["command"], "updated-command");
    for key in ["name", "args", "env", "enabled", "required", "allowedTools", "_meta"] {
        assert_eq!(updated[key], original[key], "{key}");
    }
    assert_eq!(updated["timeout"].as_f64(), Some(12.0));
}

#[test]
fn template_in_global_editor_preserves_the_server_identity() {
    let mut page = crate::state::State::default();
    page.editor = text_editor::Content::with_text(r#"[{"name":"demo","command":"before"}]"#);
    let mut draft = Draft::new(context());
    global::open(&mut draft, &page, Some(0)).unwrap();
    draft.presets.push(McpPresetDraftView {
        id: "template".into(),
        name: "Template".into(),
        tags: vec![],
        homepage: String::new(),
        documentation: String::new(),
        connection: McpConnectionDefinition::StreamableHttp {
            url: "https://example.invalid/mcp".into(),
            headers: BTreeMap::new(),
        },
        enabled: false,
    });
    draft.edit(FormMessage::Preset(0));

    let updated = global::server(&draft, &draft.global).unwrap();
    assert_eq!(updated["name"], "demo");
    assert_eq!(updated["type"], "http");
    assert_eq!(updated["enabled"], true);
    assert!(updated.get("command").is_none());
}
