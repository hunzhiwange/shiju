use super::pi::{PiMcp, PiMcpEdit, PiMcpError, PiMcpServer};
use crate::providers::{
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
    pi::PiFiles,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap, fs, io::ErrorKind, net::TcpListener, os::unix::fs::PermissionsExt,
};
use vw_api_types::{
    TenantScope,
    agent_apps_auth::AuthService,
    mcp_management::{McpConfigValue, McpConnectionDefinition, McpCredentialRef},
};

struct Fixture {
    home: tempfile::TempDir,
    _transaction_dir: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(config: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        let transaction_dir = tempfile::tempdir().unwrap();
        fs::set_permissions(transaction_dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let transactions = ProviderTransactions::new(
            transaction_dir.path(),
            ExecutionHostId::new("host").unwrap(),
        )
        .unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let fixture = Self { home, _transaction_dir: transaction_dir, transactions, tenant };
        drop(fixture.files());
        fs::write(fixture.path(), config).unwrap();
        fixture
    }

    fn path(&self) -> std::path::PathBuf {
        self.home.path().join(".pi/agent/mcp.json")
    }

    fn files(&self) -> PiFiles {
        PiFiles::open_directory(self.home.path(), None, None).unwrap()
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(self.path()).unwrap()
    }

    fn apply(&self, id: &str, edit: PiMcpEdit<'_>) -> Result<(), PiMcpError> {
        let mut files = self.files();
        PiMcp::new(&mut files.guard).unwrap().apply(
            &self.transactions,
            &self.tenant,
            id,
            edit,
            || Ok(()),
        )
    }

    fn read(&self) -> BTreeMap<String, Result<PiMcpServer, PiMcpError>> {
        PiMcp::new(&mut self.files().guard).unwrap().read().unwrap()
    }
}

#[test]
fn cc_mcp_t17_disable_retains_native_entry() {
    let fixture =
        Fixture::new(r#"{"mcpServers":{"managed":{"command":"fixture","timeout":30000}}}"#);
    fixture.apply("managed", PiMcpEdit::SetEnabled(false)).unwrap();

    let result: Value = serde_json::from_slice(&fixture.bytes()).unwrap();
    assert_eq!(
        result["mcpServers"]["managed"],
        json!({"command":"fixture","timeout":30000,"enabled":false}),
        "disabling a Pi binding must retain its connection and native options"
    );
    assert!(!fixture.read()["managed"].as_ref().unwrap().enabled);
}

#[test]
fn cc_mcp_t17_lifecycle_round_trips_and_preserves_disabled_state() {
    let fixture = Fixture::new(
        r#"{"autoEnableCodemode" : false, "mcpServers":{"unmanaged" : {"command":"fixture"}}}"#,
    );
    let project = fixture.home.path().join(".pi/mcp.json");
    let project_bytes = br#"{"mcpServers":{"managed":{"command":"project","enabled":true}}}"#;
    fs::write(&project, project_bytes).unwrap();
    let marker = fixture.home.path().join("started");
    let server = PiMcpServer {
        connection: McpConnectionDefinition::Stdio {
            command: "/bin/sh".into(),
            args: vec!["-c".into(), "touch started".into()],
            env: BTreeMap::from([(
                "FIXTURE".into(),
                McpConfigValue::Literal { value: "stdio".into() },
            )]),
            cwd: Some(fixture.home.path().to_str().unwrap().into()),
        },
        enabled: true,
        options: serde_json::from_value(json!({
            "timeout": 30000,
            "description": "fixture",
            "exposure": "direct",
            "toolExposure": {"delete_*":"hidden"},
            "oauth": {"clientName":"fixture"},
            "auth": {"mode":"fixture"},
        }))
        .unwrap(),
    };
    fixture.apply("managed", PiMcpEdit::Insert(&server)).unwrap();
    let entries = fixture.read();
    let actual = entries["managed"].as_ref().unwrap();
    assert!(actual.enabled && actual.connection == server.connection);
    assert_eq!(actual.options, server.options);
    assert!(!marker.exists());
    assert_eq!(fs::metadata(fixture.path()).unwrap().permissions().mode() & 0o777, 0o600);

    fixture.apply("managed", PiMcpEdit::SetEnabled(false)).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let http = McpConnectionDefinition::StreamableHttp {
        url: url.clone(),
        headers: BTreeMap::from([(
            "x-fixture".into(),
            McpConfigValue::Literal { value: "http".into() },
        )]),
    };
    fixture.apply("managed", PiMcpEdit::Update(&http)).unwrap();
    let entries = fixture.read();
    let actual = entries["managed"].as_ref().unwrap();
    assert!(!actual.enabled && actual.connection == http);
    assert_eq!(actual.options, server.options);
    let bytes = fixture.bytes();
    let config: Value = serde_json::from_slice(&bytes).unwrap();
    let native = &config["mcpServers"]["managed"];
    assert_eq!(native["type"], "http");
    assert_eq!(native["url"], url);
    for field in ["command", "args", "env", "cwd"] {
        assert!(native.get(field).is_none());
    }

    fixture.apply("managed", PiMcpEdit::SetEnabled(true)).unwrap();
    assert!(fixture.read()["managed"].as_ref().unwrap().enabled);
    fixture.apply("managed", PiMcpEdit::SetEnabled(false)).unwrap();
    fixture.apply("managed", PiMcpEdit::Remove).unwrap();
    let bytes = fixture.bytes();
    fixture.apply("managed", PiMcpEdit::SetEnabled(false)).unwrap();
    fixture.apply("managed", PiMcpEdit::Remove).unwrap();
    assert_eq!(fixture.bytes(), bytes);
    assert!(!fixture.read().contains_key("managed"));
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains(r#""autoEnableCodemode" : false"#));
    assert!(text.contains(r#""unmanaged" : {"command":"fixture"}"#));
    assert_eq!(fs::read(project).unwrap(), project_bytes);
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}

#[test]
fn cc_mcp_t17_import_normalizes_pi_transports_and_reports_conflicts_per_entry() {
    let fixture = Fixture::new(
        r#"{"mcpServers":{
        "dev-tools":{"command":"fixture"}, "dev_tools":{"command":"fixture","enabled":false},
        "bad.name":{"command":"fixture"}, "stdio":{"command":"fixture"},
        "http":{"type":"streamable-http","url":"http://127.0.0.1/mcp"},
        "implicit":{"url":"http://127.0.0.1/mcp","enabled":false},
        "sse":{"type":"sse","url":"http://127.0.0.1/sse"}
    }}"#,
    );
    let before = fixture.bytes();
    let entries = fixture.read();
    for id in ["dev-tools", "dev_tools"] {
        assert!(matches!(entries[id], Err(PiMcpError::File(WriteError::Conflict))));
    }
    assert!(matches!(entries["bad.name"], Err(PiMcpError::InvalidDefinition)));
    assert!(matches!(entries["sse"], Err(PiMcpError::UnsupportedTransport)));
    let stdio = entries["stdio"].as_ref().unwrap();
    assert!(stdio.enabled && matches!(stdio.connection, McpConnectionDefinition::Stdio { .. }));
    let http = entries["http"].as_ref().unwrap();
    let implicit = entries["implicit"].as_ref().unwrap();
    assert!(matches!(http.connection, McpConnectionDefinition::StreamableHttp { .. }));
    assert!(!implicit.enabled && implicit.connection == http.connection);
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn cc_mcp_t17_rejected_writes_leave_native_file_unchanged() {
    let fixture = Fixture::new(r#"{"mcpServers":{"dev-tools":{"command":"user-owned"}}}"#);
    let before = fixture.bytes();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let mut server = PiMcpServer {
        connection: McpConnectionDefinition::StreamableHttp {
            url: url.clone(),
            headers: BTreeMap::new(),
        },
        enabled: true,
        options: Default::default(),
    };
    for id in ["dev-tools", "dev_tools"] {
        assert_eq!(
            fixture.apply(id, PiMcpEdit::Insert(&server)),
            Err(PiMcpError::File(WriteError::Conflict))
        );
    }
    assert_eq!(
        fixture.apply("bad.name", PiMcpEdit::Insert(&server)),
        Err(PiMcpError::InvalidDefinition)
    );
    server.connection = McpConnectionDefinition::Sse { url: url.clone(), headers: BTreeMap::new() };
    assert_eq!(
        fixture.apply("new", PiMcpEdit::Insert(&server)),
        Err(PiMcpError::UnsupportedTransport)
    );
    server.connection = McpConnectionDefinition::StreamableHttp {
        url,
        headers: BTreeMap::from([(
            "authorization".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::CodexOauth,
                    account_id: "fixture".into(),
                },
            },
        )]),
    };
    assert_eq!(
        fixture.apply("new", PiMcpEdit::Insert(&server)),
        Err(PiMcpError::CredentialUnavailable)
    );
    let mut files = fixture.files();
    assert_eq!(
        PiMcp::new(&mut files.guard).unwrap().apply(
            &fixture.transactions,
            &fixture.tenant,
            "dev-tools",
            PiMcpEdit::SetEnabled(false),
            || Err(WriteError::Conflict),
        ),
        Err(PiMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fixture.bytes(), before);
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}
