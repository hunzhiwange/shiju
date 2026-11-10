use super::gemini::{GeminiMcp, GeminiMcpEdit, GeminiMcpError};
use crate::providers::live::{
    commit::{ExecutionHostId, ProviderTransactions},
    patch_json::object_value,
    write_guard::{WriteError, lock_app},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap, fs, io::ErrorKind, net::TcpListener, os::unix::fs::PermissionsExt,
};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    mcp_management::{McpConfigValue, McpConnectionDefinition, McpCredentialRef},
};

struct Fixture {
    _root: tempfile::TempDir,
    live: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(settings: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let transactions =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::write(live.path().join("settings.json"), settings).unwrap();
        Self {
            _root: root,
            live,
            transactions,
            tenant: TenantScope::new("platform", "company").unwrap(),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(self.live.path().join("settings.json")).unwrap()
    }

    fn settings(&self) -> Value {
        object_value(&self.bytes()).unwrap()
    }

    fn apply(&self, edit: GeminiMcpEdit<'_>) -> Result<(), GeminiMcpError> {
        let mut guard = lock_app(AppId::Gemini, self.live.path()).unwrap();
        GeminiMcp::new(&mut guard).unwrap().apply(
            &self.transactions,
            &self.tenant,
            "managed",
            edit,
            || Ok(()),
        )
    }

    fn assert_import(&self, expected: &McpConnectionDefinition) {
        let mut guard = lock_app(AppId::Gemini, self.live.path()).unwrap();
        let servers = GeminiMcp::new(&mut guard).unwrap().read().unwrap();
        assert!(servers["managed"].as_ref().is_ok_and(|actual| actual == expected));
    }
}

#[test]
fn cc_mcp_t13_http_writes_http_url() {
    let fixture = Fixture::new(r#"{"theme":"dark"}"#);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let headers =
        BTreeMap::from([("x-fixture".into(), McpConfigValue::Literal { value: "http".into() })]);
    let connection = McpConnectionDefinition::StreamableHttp { url: url.clone(), headers };
    fixture.apply(GeminiMcpEdit::Insert(&connection)).unwrap();

    let settings = fixture.settings();
    assert_eq!(settings["mcpServers"]["managed"]["httpUrl"], json!(url));
    assert_eq!(
        settings["mcpServers"]["managed"],
        json!({"httpUrl": url, "headers": {"x-fixture": "http"}})
    );
    assert_eq!(settings["theme"], "dark");
    fixture.assert_import(&connection);
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}

#[test]
fn cc_mcp_t13_lifecycle_preserves_unmanaged_settings() {
    let fixture =
        Fixture::new(r#"{"theme" : "dark", "mcpServers":{"unmanaged" : {"command":"keep"}}}"#);
    let executable = fixture.live.path().join("fixture");
    let marker = fixture.live.path().join("started");
    fs::write(&executable, "#!/bin/sh\ntouch \"$1\"\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let stdio = McpConnectionDefinition::Stdio {
        command: executable.to_str().unwrap().into(),
        args: vec![marker.to_str().unwrap().into()],
        env: BTreeMap::from([(
            "FIXTURE".into(),
            McpConfigValue::Literal { value: "stdio".into() },
        )]),
        cwd: Some(fixture.live.path().to_str().unwrap().into()),
    };
    fixture.apply(GeminiMcpEdit::Insert(&stdio)).unwrap();
    fixture.assert_import(&stdio);
    assert!(!marker.exists());
    let after_insert = String::from_utf8(fixture.bytes()).unwrap();
    assert!(after_insert.contains(r#""theme" : "dark""#));
    assert!(after_insert.contains(r#""unmanaged" : {"command":"keep"}"#));

    // Add Gemini-only options to the managed entry and verify a transport change retains them.
    let mut settings = fixture.settings();
    settings["mcpServers"]["managed"]["timeout"] = json!(75000);
    fs::write(fixture.live.path().join("settings.json"), serde_json::to_vec(&settings).unwrap())
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/sse", listener.local_addr().unwrap());
    let sse = McpConnectionDefinition::Sse { url: url.clone(), headers: Default::default() };
    fixture.apply(GeminiMcpEdit::Update(&sse)).unwrap();
    assert_eq!(fixture.settings()["mcpServers"]["managed"], json!({"url":url,"timeout":75000}));
    fixture.assert_import(&sse);
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);

    fixture.apply(GeminiMcpEdit::Remove).unwrap(); // Disable.
    let disabled = fixture.bytes();
    fixture.apply(GeminiMcpEdit::Remove).unwrap(); // Delete while disabled.
    assert_eq!(fixture.bytes(), disabled);
    assert_eq!(
        fixture.settings(),
        json!({"theme":"dark","mcpServers":{"unmanaged":{"command":"keep"}}})
    );
}

#[test]
fn cc_mcp_t13_import_reports_unsupported_and_ambiguous_transports_per_server() {
    let fixture = Fixture::new(
        r#"{"mcpServers":{
        "unsupported":{"type":"websocket","url":"http://127.0.0.1/sse"},
        "ambiguous":{"url":"http://127.0.0.1/sse","httpUrl":"http://127.0.0.1/mcp"},
        "valid":{"type":"sse","url":"http://127.0.0.1/sse"}
    }}"#,
    );
    let before = fixture.bytes();
    let mut guard = lock_app(AppId::Gemini, fixture.live.path()).unwrap();
    let entries = GeminiMcp::new(&mut guard).unwrap().read().unwrap();
    assert!(matches!(entries["unsupported"], Err(GeminiMcpError::UnsupportedTransport)));
    assert!(matches!(entries["ambiguous"], Err(GeminiMcpError::InvalidDefinition)));
    assert!(matches!(entries["valid"], Ok(McpConnectionDefinition::Sse { .. })));
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn cc_mcp_t13_conflict_and_unresolved_credential_leave_settings_unchanged() {
    let fixture = Fixture::new(r#"{"mcpServers":{"managed":{"command":"user-owned"}}}"#);
    let before = fixture.bytes();
    let url = "http://127.0.0.1/mcp".to_owned();
    let connection =
        McpConnectionDefinition::StreamableHttp { url: url.clone(), headers: Default::default() };
    assert_eq!(
        fixture.apply(GeminiMcpEdit::Insert(&connection)),
        Err(GeminiMcpError::File(WriteError::Conflict))
    );

    let headers = BTreeMap::from([(
        "authorization".into(),
        McpConfigValue::Credential {
            reference: McpCredentialRef {
                service: AuthService::CodexOauth,
                account_id: "fixture".into(),
            },
        },
    )]);
    let unresolved = McpConnectionDefinition::StreamableHttp { url, headers };
    assert_eq!(
        fixture.apply(GeminiMcpEdit::Update(&unresolved)),
        Err(GeminiMcpError::CredentialUnavailable)
    );
    assert_eq!(fixture.bytes(), before);
}
