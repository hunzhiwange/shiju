use super::{Edit, OpenCodeMcpError, apply, read};
use crate::providers::{
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        patch_json::object_value,
        write_guard::WriteError,
    },
    opencode::OpenCodeFiles,
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{
    TenantScope,
    mcp_management::{McpConfigValue, McpConnectionDefinition},
};

const ORIGINAL: &str = r#"{
    // retain this comment
    "provider": { "custom": {} },
    "mcp": { "user": { "type": "local", "command": ["keep"] }, },
}"#;

struct Fixture {
    home: tempfile::TempDir,
    _store: tempfile::TempDir,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join(".config/opencode");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("opencode.jsonc"), ORIGINAL).unwrap();
        fs::write(dir.join("opencode.json"), "{}").unwrap();
        let store = tempfile::tempdir().unwrap();
        fs::set_permissions(store.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(store.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        Self { home, _store: store, tx, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn path(&self) -> PathBuf {
        self.home.path().join(".config/opencode/opencode.jsonc")
    }

    fn apply(&self, id: &str, edit: Edit<'_>) -> Result<(), OpenCodeMcpError> {
        let mut files = OpenCodeFiles::open(self.home.path()).unwrap();
        apply(&mut files, &self.tx, &self.tenant, id, edit, || Ok(()))
    }
}

fn stdio() -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio {
        command: "unused-fixture-command".into(),
        args: vec!["--mode".into(), "test with spaces".into()],
        env: [("MCP_MODE".into(), McpConfigValue::Literal { value: "test".into() })].into(),
        cwd: None,
    }
}

#[test]
fn cc_mcp_t15_managed_jsonc_round_trips_through_lifecycle() {
    let fixture = Fixture::new();
    let local = stdio();
    assert_eq!(
        fixture.apply("managed", Edit::Insert(&local)),
        Ok(()),
        "OpenCode must publish a managed stdio definition"
    );

    let doc = object_value(&fs::read(fixture.path()).unwrap()).unwrap();
    assert_eq!(
        doc["mcp"]["managed"],
        json!({
            "type": "local",
            "command": ["unused-fixture-command", "--mode", "test with spaces"],
            "environment": {"MCP_MODE": "test"},
            "enabled": true
        })
    );
    let imported = read(fixture.home.path()).unwrap();
    assert!(imported["managed"].as_ref().unwrap().connection == local);
    assert!(imported["managed"].as_ref().unwrap().enabled);

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let remote = McpConnectionDefinition::Sse {
        url: url.clone(),
        headers: [("X-MCP-Mode".into(), McpConfigValue::Literal { value: "test".into() })].into(),
    };
    fixture.apply("managed", Edit::Update(&remote)).unwrap();
    let doc = object_value(&fs::read(fixture.path()).unwrap()).unwrap();
    assert_eq!(
        doc["mcp"]["managed"],
        json!({"type": "remote", "url": url, "headers": {"X-MCP-Mode": "test"}, "enabled": true})
    );
    fixture.apply("managed", Edit::SetEnabled(false)).unwrap();
    fixture.apply("managed", Edit::Update(&remote)).unwrap();
    let imported = read(fixture.home.path()).unwrap();
    let server = imported["managed"].as_ref().unwrap();
    assert!(server.connection == remote);
    assert!(!server.enabled);
    assert!(imported["user"].as_ref().unwrap().enabled);
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );

    fixture.apply("managed", Edit::Remove).unwrap();
    assert!(!read(fixture.home.path()).unwrap().contains_key("managed"));
    let after = fs::read_to_string(fixture.path()).unwrap();
    for expected in [
        "// retain this comment",
        r#""provider": { "custom": {} }"#,
        r#""user": { "type": "local", "command": ["keep"] }"#,
    ] {
        assert!(after.contains(expected));
    }
    assert_eq!(fs::read(fixture.path().with_extension("json")).unwrap(), b"{}");
}

#[test]
fn cc_mcp_t15_same_name_keeps_user_configuration() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.apply("user", Edit::Insert(&stdio())),
        Err(OpenCodeMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fs::read(fixture.path()).unwrap(), ORIGINAL.as_bytes());
}

#[test]
fn cc_mcp_t15_unrepresentable_fields_and_unresolved_credentials_leave_file_unchanged() {
    use vw_api_types::{agent_apps_auth::AuthService, mcp_management::McpCredentialRef};

    let fixture = Fixture::new();
    let http = McpConnectionDefinition::StreamableHttp {
        url: "http://127.0.0.1:1/mcp".into(),
        headers: Default::default(),
    };
    assert_eq!(
        fixture.apply("managed", Edit::Insert(&http)),
        Err(OpenCodeMcpError::UnsupportedTransport)
    );
    let mut with_cwd = stdio();
    let McpConnectionDefinition::Stdio { cwd, .. } = &mut with_cwd else { unreachable!() };
    *cwd = Some("/fixture".into());
    assert_eq!(
        fixture.apply("managed", Edit::Insert(&with_cwd)),
        Err(OpenCodeMcpError::UnsupportedField("cwd"))
    );
    let mut local = stdio();
    let McpConnectionDefinition::Stdio { env, .. } = &mut local else { unreachable!() };
    env.insert(
        "FIXTURE_KEY".into(),
        McpConfigValue::Credential {
            reference: McpCredentialRef {
                service: AuthService::CodexOauth,
                account_id: "fixture".into(),
            },
        },
    );
    assert_eq!(
        fixture.apply("managed", Edit::Insert(&local)),
        Err(OpenCodeMcpError::CredentialUnavailable)
    );
    assert_eq!(fs::read(fixture.path()).unwrap(), ORIGINAL.as_bytes());
}

#[test]
fn cc_mcp_t15_import_reports_each_unrepresentable_entry() {
    let fixture = Fixture::new();
    let config = json!({
        "mcp": {
            "ok": {"command": ["fixture"]},
            "empty-command": {"type": "local", "command": []},
            "bad-enabled": {"type": "local", "command": ["fixture"], "enabled": "false"},
            "http-variant": {"type": "streamable_http", "url": "http://127.0.0.1:1/mcp"},
            "oauth": {"type": "remote", "url": "http://127.0.0.1:1/mcp", "oauth": false}
        }
    });
    fs::write(fixture.path(), config.to_string()).unwrap();

    let servers = read(fixture.home.path()).unwrap();
    assert!(servers["ok"].as_ref().unwrap().enabled);
    for key in ["empty-command", "bad-enabled"] {
        assert!(matches!(servers[key], Err(OpenCodeMcpError::InvalidDefinition)));
    }
    assert!(matches!(servers["http-variant"], Err(OpenCodeMcpError::UnsupportedTransport)));
    assert!(matches!(servers["oauth"], Err(OpenCodeMcpError::UnsupportedField("native options"))));
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), config.to_string());
}

#[test]
fn cc_mcp_t15_json_fallback_rechecks_precedence_before_publish() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.path()).unwrap();
    let mut files = OpenCodeFiles::open(fixture.home.path()).unwrap();
    assert_eq!(
        apply(&mut files, &fixture.tx, &fixture.tenant, "managed", Edit::Insert(&stdio()), || {
            fs::write(fixture.path(), ORIGINAL).unwrap();
            Ok(())
        }),
        Err(OpenCodeMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fs::read(fixture.path()).unwrap(), ORIGINAL.as_bytes());
    assert_eq!(fs::read(fixture.path().with_extension("json")).unwrap(), b"{}");
}

#[test]
fn cc_mcp_t15_default_file_is_json_when_no_config_exists() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.path()).unwrap();
    let json = fixture.path().with_extension("json");
    fs::remove_file(&json).unwrap();
    assert!(read(fixture.home.path()).unwrap().is_empty());
    assert!(!json.exists());
    fixture.apply("managed", Edit::Insert(&stdio())).unwrap();
    assert!(json.exists());
    assert!(!fixture.path().exists());
    assert!(read(fixture.home.path()).unwrap()["managed"].as_ref().unwrap().enabled);
}
