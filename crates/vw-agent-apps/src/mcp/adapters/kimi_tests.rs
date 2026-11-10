use super::kimi;
use crate::providers::{
    kimi::KimiFiles,
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
};
use serde_json::Value;
use std::{
    collections::BTreeMap, fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf,
};
use vw_api_types::{TenantScope, mcp_management::McpConnectionDefinition};

struct Fixture {
    home: tempfile::TempDir,
    _store: tempfile::TempDir,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(config: Option<&str>) -> Self {
        let home = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        fs::set_permissions(store.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(store.path(), ExecutionHostId::new("test-host").unwrap())
                .unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let fixture = Self { home, _store: store, tx, tenant };
        if let Some(config) = config {
            fs::create_dir(fixture.path().parent().unwrap()).unwrap();
            fs::write(fixture.path(), config).unwrap();
        }
        fixture
    }

    fn path(&self) -> PathBuf {
        self.home.path().join(".kimi-code/mcp.json")
    }

    fn apply(&self, id: &str, edit: kimi::Edit<'_>) -> Result<(), kimi::KimiMcpError> {
        let mut files = KimiFiles::open(self.home.path()).unwrap();
        kimi::apply(&mut files.guard, &self.tx, &self.tenant, id, edit, || Ok(()))
    }

    fn read(
        &self,
    ) -> Result<BTreeMap<String, Result<kimi::ImportedServer, kimi::KimiMcpError>>, WriteError>
    {
        let mut files = KimiFiles::open(self.home.path()).unwrap();
        kimi::read(&mut files.guard)
    }

    fn check_import(&self, id: &str, connection: &McpConnectionDefinition, enabled: bool) {
        let servers = self.read().unwrap();
        let imported = servers[id].as_ref().unwrap();
        assert!(
            imported.connection == *connection,
            "native import must match the projected connection"
        );
        assert_eq!(imported.enabled, enabled);
    }

    fn native(&self) -> Value {
        serde_json::from_slice(&fs::read(self.path()).unwrap()).unwrap()
    }
}

#[test]
fn cc_mcp_t19_sse_writes_native_transport() {
    let fixture = Fixture::new(None);
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let url = format!("http://{}/events", listener.local_addr().unwrap());
    let connection = McpConnectionDefinition::Sse { url, headers: Default::default() };
    fixture.apply("managed", kimi::Edit::Insert(&connection)).unwrap();
    let native = fixture.native();
    assert_eq!(native["mcpServers"]["managed"]["transport"], "sse");
    assert!(native["mcpServers"]["managed"].get("type").is_none());
}

#[test]
fn cc_mcp_t19_updates_and_revokes_only_the_managed_entry() {
    use vw_api_types::mcp_management::McpConfigValue::Literal;

    let fixture = Fixture::new(Some(
        r#"{
  "custom": {"value": "keep"},
  "mcpServers": {"user": {"command": "keep", "enabled": false}}
}
"#,
    ));
    let connection = McpConnectionDefinition::Stdio {
        command: "/bin/sh".into(),
        args: vec!["-c".into(), "touch executed".into()],
        env: [("MCP_MODE".into(), Literal { value: "fixture".into() })].into(),
        cwd: Some(fixture.home.path().to_str().unwrap().into()),
    };
    fixture.apply("managed", kimi::Edit::Insert(&connection)).unwrap();
    fixture.check_import("managed", &connection, true);
    assert_eq!(
        fixture.apply("user", kimi::Edit::Insert(&connection)),
        Err(kimi::KimiMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fs::metadata(fixture.path()).unwrap().permissions().mode() & 0o777, 0o600);

    // Simulate a native tool adding options to the managed entry.
    let mut native = fixture.native();
    native["mcpServers"]["managed"]["deferred"] = Value::Bool(true);
    native["mcpServers"]["managed"]["toolTimeoutMs"] = 1234.into();
    fs::write(fixture.path(), serde_json::to_vec_pretty(&native).unwrap()).unwrap();

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let sse = McpConnectionDefinition::Sse {
        url: format!("http://{}/events", listener.local_addr().unwrap()),
        headers: [("X-Mode".into(), Literal { value: "fixture".into() })].into(),
    };
    fixture.apply("managed", kimi::Edit::Update(&sse)).unwrap();
    fixture.check_import("managed", &sse, true);
    let native = fixture.native();
    let row = &native["mcpServers"]["managed"];
    assert_eq!(row["transport"], "sse");
    assert_eq!(row["deferred"], true);
    assert_eq!(row["toolTimeoutMs"], 1234);
    for field in ["command", "args", "env", "cwd", "type"] {
        assert!(row.get(field).is_none());
    }

    let http = McpConnectionDefinition::StreamableHttp {
        url: format!("http://{}/mcp", listener.local_addr().unwrap()),
        headers: Default::default(),
    };
    fixture.apply("managed", kimi::Edit::Update(&http)).unwrap();
    fixture.check_import("managed", &http, true);
    let native = fixture.native();
    let row = &native["mcpServers"]["managed"];
    assert!(row.get("transport").is_none());
    assert!(row.get("headers").is_none());
    assert_eq!(row["deferred"], true);
    assert_eq!(native["custom"]["value"], "keep");
    assert_eq!(native["mcpServers"]["user"]["command"], "keep");
    assert_eq!(native["mcpServers"]["user"]["enabled"], false);

    fixture.apply("managed", kimi::Edit::Remove).unwrap(); // disable
    assert!(!fixture.read().unwrap().contains_key("managed"));
    let disabled = fs::read(fixture.path()).unwrap();
    fixture.apply("managed", kimi::Edit::Remove).unwrap(); // delete after disable
    assert_eq!(fs::read(fixture.path()).unwrap(), disabled);
    assert_eq!(fixture.native()["mcpServers"]["user"], native["mcpServers"]["user"]);
    assert!(!fixture.home.path().join("executed").exists());
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(listener.accept(), Err(err) if err.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn cc_mcp_t19_imports_legacy_selectors_and_isolates_invalid_entries() {
    let fixture = Fixture::new(Some(
        r#"{"mcpServers":{
          "http":{"transport":"streamable-http","url":"https://example.invalid/mcp","enabled":false},
          "sse":{"type":"sse","url":"https://example.invalid/events","deferred":true},
          "unknown":{"transport":"websocket","url":"https://example.invalid"},
          "bad":{"transport":false,"command":"test"},
          "bad_fields":{"transport":"sse","command":"test","url":"https://example.invalid"},
          "bad_enabled":{"command":"test","enabled":"false"}
        }}"#,
    ));
    let imported = fixture.read().unwrap();
    let http = imported["http"].as_ref().unwrap();
    assert!(!http.enabled);
    assert!(matches!(http.connection, McpConnectionDefinition::StreamableHttp { .. }));
    let sse = imported["sse"].as_ref().unwrap();
    assert!(sse.enabled);
    assert!(matches!(sse.connection, McpConnectionDefinition::Sse { .. }));
    assert_eq!(imported["unknown"].as_ref().err(), Some(&kimi::KimiMcpError::UnsupportedTransport));
    for id in ["bad", "bad_fields", "bad_enabled"] {
        assert_eq!(imported[id].as_ref().err(), Some(&kimi::KimiMcpError::InvalidDefinition));
    }
}

#[test]
fn cc_mcp_t19_rejects_bad_maps_and_preserves_source_bytes() {
    for invalid in [
        "{invalid",
        "[]",
        r#"{"mcpServers":null}"#,
        r#"{"mcpServers":[]}"#,
        r#"{"mcpServers":{},"mcpServers":{}}"#,
        r#"{"mcpServers":{},}"#,
    ] {
        let fixture = Fixture::new(Some(invalid));
        assert_eq!(fixture.read().err(), Some(WriteError::InvalidResource));
        assert_eq!(
            fixture.apply("managed", kimi::Edit::Remove),
            Err(kimi::KimiMcpError::File(WriteError::InvalidResource))
        );
        assert_eq!(fs::read_to_string(fixture.path()).unwrap(), invalid);
    }
    let fixture = Fixture::new(None);
    fixture.apply("managed", kimi::Edit::Remove).unwrap();
    assert!(fixture.read().unwrap().is_empty());
    assert!(!fixture.path().exists());
}

#[test]
fn cc_mcp_t19_rejects_unresolved_credentials_and_failed_preflight() {
    use vw_api_types::{
        agent_apps_auth::AuthService,
        mcp_management::{McpConfigValue, McpCredentialRef},
    };

    let fixture = Fixture::new(Some("{}"));
    let connection = McpConnectionDefinition::Stdio {
        command: "mcp-test".into(),
        args: Vec::new(),
        env: [(
            "MCP_TOKEN".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::CodexOauth,
                    account_id: "fixture".into(),
                },
            },
        )]
        .into(),
        cwd: None,
    };
    assert_eq!(
        fixture.apply("managed", kimi::Edit::Insert(&connection)),
        Err(kimi::KimiMcpError::CredentialUnavailable)
    );

    let mut files = KimiFiles::open(fixture.home.path()).unwrap();
    let connection = McpConnectionDefinition::Stdio {
        command: "mcp-test".into(),
        args: Vec::new(),
        env: Default::default(),
        cwd: None,
    };
    assert_eq!(
        kimi::apply(
            &mut files.guard,
            &fixture.tx,
            &fixture.tenant,
            "managed",
            kimi::Edit::Insert(&connection),
            || Err(WriteError::Conflict),
        ),
        Err(kimi::KimiMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), "{}");
}
