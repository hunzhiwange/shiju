use super::*;
use desktop_provider_apps::AgentAppsContext;
use {vw_api_app_auth::AuthService, vw_api_transport::TenantScope};
fn context() -> crate::management_context::Context {
    crate::management_context::Context {
        endpoint: AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: TenantScope::new("10000", "100000001").unwrap(),
            project: None,
        },
        host: "host".into(),
        credentials_generation: 1,
    }
}
fn draft() -> Draft {
    let mut draft = Draft::new(context(), None);
    draft.id = "demo".into();
    draft.name = "中文连接 😀".into();
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
    draft.finish(&Err("修订冲突".into()));
    assert!(!draft.saving);
    assert_eq!(draft.error.as_deref(), Some("修订冲突"));
    assert!(draft.editor.text().contains("private-secret"));
}
