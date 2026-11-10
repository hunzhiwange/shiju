use super::codex::{self, CodexEdit, CodexError, ImportedServer};
use crate::{
    mcp::validation::parse_native,
    providers::live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::{WriteError, lock_app},
    },
};
use serde_json::json;
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    mcp_management::{McpConfigValue, McpConnectionDefinition, McpCredentialRef},
};

const GOLDEN: &str =
    include_str!("../../../upstream/src-tauri/tests/golden/snapshots/mcp/codex-config.toml");

struct Fixture {
    root: tempfile::TempDir,
    live: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(before: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::write(live.path().join("config.toml"), before).unwrap();
        fs::write(live.path().join("auth.json"), b"fixture").unwrap();
        let transactions =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        Self { root, live, transactions, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn edit(&self, id: &str, edit: CodexEdit<'_>) -> Result<(), CodexError> {
        let mut guard = lock_app(AppId::Codex, self.live.path()).unwrap();
        codex::apply(&mut guard, &self.transactions, &self.tenant, id, edit, || Ok(()))
    }

    fn read(&self, id: &str) -> Result<Option<ImportedServer>, CodexError> {
        let mut guard = lock_app(AppId::Codex, self.live.path()).unwrap();
        codex::read(&mut guard)?.remove(id).transpose()
    }

    fn bytes(&self) -> String {
        fs::read_to_string(self.live.path().join("config.toml")).unwrap()
    }
}

#[test]
fn cc_mcp_t12_guarded_lifecycle_matches_golden_and_reads_back() {
    let before = GOLDEN.split_once("\n[mcp_servers.http-server]").unwrap().0;
    let fixture = Fixture::new(before);
    let stdio = parse_native(json!({
        "command": "example-mcp-server", "args": ["--stdio"], "env": {"EXAMPLE_TOKEN": "mcp-token"}
    }))
    .unwrap();
    let http = parse_native(json!({
        "url": "https://mcp.example.com/mcp", "headers": {"Authorization": "Bearer mcp-header"}
    }))
    .unwrap();

    fixture.edit("http-server", CodexEdit::Insert(&http)).unwrap();
    fixture.edit("stdio-server", CodexEdit::Insert(&stdio)).unwrap();
    assert_eq!(fixture.bytes(), GOLDEN, "native fields must match the frozen Codex projection");
    for (id, definition) in [("http-server", &http), ("stdio-server", &stdio)] {
        let entry = fixture.read(id).unwrap().unwrap();
        assert!(entry.enabled && &entry.connection == definition);
    }

    let updated = parse_native(json!({"command": "new", "cwd": "/fixture"})).unwrap();
    fixture.edit("http-server", CodexEdit::Update(&updated)).unwrap();
    let entry = fixture.read("http-server").unwrap().unwrap();
    assert!(entry.enabled && entry.connection == updated);
    assert!(!fixture.bytes().contains("Bearer mcp-header"));

    fixture.edit("http-server", CodexEdit::Remove).unwrap(); // disable
    assert!(fixture.read("http-server").unwrap().is_none());
    let disabled = fixture.bytes();
    fixture.edit("http-server", CodexEdit::Remove).unwrap(); // delete
    assert_eq!(fixture.bytes(), disabled);
    assert!(disabled.starts_with(before));
    assert!(fixture.read("stdio-server").unwrap().is_some());
    assert_eq!(fs::read(fixture.live.path().join("auth.json")).unwrap(), b"fixture");
    assert!(fixture.transactions.pending(AppId::Codex).unwrap().is_none());
    assert!(fixture.root.path().is_dir());
}

#[test]
fn cc_mcp_t12_legacy_import_and_inline_update_preserve_unowned_entries() {
    let before = r#"
mcp_servers = { managed = { command = "old", enabled = false }, user = { command = "user" } }
[mcp.servers.legacy]
type = "http"
url = "https://legacy.example/mcp"
headers = { X-Example = "fixture" }
"#;
    let fixture = Fixture::new(before);
    let legacy = parse_native(json!({
        "url": "https://legacy.example/mcp", "headers": {"X-Example": "fixture"}
    }))
    .unwrap();
    assert!(fixture.read("legacy").unwrap().unwrap().connection == legacy);
    assert!(!fixture.read("managed").unwrap().unwrap().enabled);
    assert_eq!(
        fixture.edit("legacy", CodexEdit::Insert(&legacy)),
        Err(CodexError::Write(WriteError::Conflict))
    );
    assert_eq!(fixture.bytes(), before);

    let updated = parse_native(json!({
        "url": "https://example.invalid/mcp", "headers": {"X-Test": "fixture"}
    }))
    .unwrap();
    fixture.edit("managed", CodexEdit::Update(&updated)).unwrap();
    let entry = fixture.read("managed").unwrap().unwrap();
    assert!(entry.enabled && entry.connection == updated);
    fixture.edit("managed", CodexEdit::Remove).unwrap();
    assert!(fixture.read("managed").unwrap().is_none());
    assert!(fixture.read("user").unwrap().is_some());
    assert!(fixture.read("legacy").unwrap().is_some());
}

#[test]
fn cc_mcp_t12_invalid_config_and_unsupported_transport_leave_file_intact() {
    let stdio = parse_native(json!({"command": "fixture"})).unwrap();
    for before in ["[broken", "mcp_servers = 5", "[mcp_servers]\nowned = false\n"] {
        let fixture = Fixture::new(before);
        assert_eq!(
            fixture.edit("owned", CodexEdit::Update(&stdio)),
            Err(CodexError::Write(WriteError::InvalidResource))
        );
        assert_eq!(fixture.bytes(), before);
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let sse = parse_native(json!({
        "type": "sse", "url": format!("http://{}/sse", listener.local_addr().unwrap())
    }))
    .unwrap();
    let before = "# keep\n";
    let fixture = Fixture::new(before);
    assert_eq!(
        fixture.edit("remote", CodexEdit::Insert(&sse)),
        Err(CodexError::UnsupportedTransport)
    );
    assert_eq!(fixture.bytes(), before);
    listener.set_nonblocking(true).unwrap();
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[test]
fn cc_mcp_t12_unresolved_credentials_and_conflicting_user_entries_are_not_written() {
    let before = "[mcp_servers.user]\ncommand = 'user'\n";
    let fixture = Fixture::new(before);
    let stdio = parse_native(json!({"command": "fixture"})).unwrap();
    assert_eq!(
        fixture.edit("user", CodexEdit::Insert(&stdio)),
        Err(CodexError::Write(WriteError::Conflict))
    );

    let credential = McpConnectionDefinition::Stdio {
        command: "fixture".into(),
        args: vec![],
        cwd: None,
        env: [(
            "TOKEN".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::CodexOauth,
                    account_id: "fixture".into(),
                },
            },
        )]
        .into(),
    };
    assert_eq!(
        fixture.edit("managed", CodexEdit::Insert(&credential)),
        Err(CodexError::UnresolvedCredential)
    );
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn cc_mcp_t12_native_credential_extensions_are_not_transferred_to_new_url() {
    let before = r#"
[mcp_servers.owned]
url = "https://old.example/mcp"
env_http_headers = { X-Token = "MCP_TOKEN" }

[mcp_servers.other]
command = "fixture"
"#;
    let fixture = Fixture::new(before);
    let updated = parse_native(json!({"url": "https://new.example/mcp"})).unwrap();

    assert!(matches!(fixture.read("owned"), Err(CodexError::UnsupportedField)));
    assert!(fixture.read("other").unwrap().is_some());
    assert_eq!(
        fixture.edit("owned", CodexEdit::Update(&updated)),
        Err(CodexError::Write(WriteError::InvalidResource))
    );
    assert_eq!(fixture.bytes(), before);
}
