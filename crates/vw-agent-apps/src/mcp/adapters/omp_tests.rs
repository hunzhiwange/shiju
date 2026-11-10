use super::omp::{OmpMcp, OmpMcpEdit, OmpMcpEntry, OmpMcpError};
use crate::providers::{
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        patch_json::object_value,
        write_guard::{WriteError, lock_app},
    },
    paths,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap, fs, io::ErrorKind, net::TcpListener, os::unix::fs::PermissionsExt,
    path::PathBuf,
};
use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    mcp_management::{McpConfigValue, McpConnectionDefinition, McpCredentialRef},
};

struct Fixture {
    _home: tempfile::TempDir,
    _store: tempfile::TempDir,
    directory: PathBuf,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(document: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        fs::set_permissions(store.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let transactions =
            ProviderTransactions::new(store.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let directory: PathBuf =
            paths::read(home.path(), AppId::Omp, None, &BTreeMap::new()).unwrap().directory.into();
        assert_eq!(directory, home.path().join(".omp/agent"));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("mcp.json"), document).unwrap();
        Self {
            _home: home,
            _store: store,
            directory,
            transactions,
            tenant: TenantScope::new("platform", "company").unwrap(),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(self.directory.join("mcp.json")).unwrap()
    }

    fn document(&self) -> Value {
        object_value(&self.bytes()).unwrap()
    }

    fn apply(&self, edit: OmpMcpEdit<'_>) -> Result<(), OmpMcpError> {
        let mut guard = lock_app(AppId::Omp, &self.directory).unwrap();
        OmpMcp::new(&mut guard).unwrap().apply(
            &self.transactions,
            &self.tenant,
            "managed",
            edit,
            || Ok(()),
        )
    }

    fn read(&self) -> BTreeMap<String, Result<OmpMcpEntry, OmpMcpError>> {
        let mut guard = lock_app(AppId::Omp, &self.directory).unwrap();
        OmpMcp::new(&mut guard).unwrap().read().unwrap()
    }

    fn assert_import(&self, expected: &McpConnectionDefinition, enabled: bool) {
        let entries = self.read();
        let entry = entries["managed"].as_ref().unwrap();
        assert!(&entry.connection == expected, "import must preserve the projected connection");
        assert_eq!(entry.enabled, enabled);
    }
}

#[test]
fn cc_mcp_t22_lifecycle_round_trips_and_preserves_unmanaged_policy() {
    let fixture = Fixture::new(
        r#"{
        "autoEnableCodemode" : false,
        "enabledServers": ["allow"],
        "disabledServers": ["deny"],
        "mcpServers": {"unmanaged" : {"command":"keep"}}
    }"#,
    );
    let executable = fixture.directory.join("fixture");
    let marker = fixture.directory.join("started");
    fs::write(&executable, "#!/bin/sh\ntouch \"$1\"\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let stdio = McpConnectionDefinition::Stdio {
        command: executable.to_str().unwrap().into(),
        args: vec![marker.to_str().unwrap().into()],
        env: BTreeMap::from([(
            "FIXTURE".into(),
            McpConfigValue::Literal { value: "stdio".into() },
        )]),
        cwd: Some(fixture.directory.to_str().unwrap().into()),
    };
    fixture.apply(OmpMcpEdit::Insert(&stdio)).unwrap();
    assert!(
        String::from_utf8(fixture.bytes()).unwrap().contains(r#""autoEnableCodemode" : false"#)
    );
    fixture.assert_import(&stdio, true);
    assert!(!marker.exists());

    let mut document = fixture.document();
    document["mcpServers"]["managed"]["timeout"] = json!(30);
    document["mcpServers"]["managed"]["toolExposure"] = json!({"write_*":"hidden"});
    document["enabledServers"] = json!(["allow", "managed", "also-allow"]);
    fs::write(fixture.directory.join("mcp.json"), serde_json::to_vec(&document).unwrap()).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let http = McpConnectionDefinition::StreamableHttp {
        url: format!("http://{}/mcp", listener.local_addr().unwrap()),
        headers: BTreeMap::from([(
            "x-fixture".into(),
            McpConfigValue::Literal { value: "http".into() },
        )]),
    };
    fixture.apply(OmpMcpEdit::Update { connection: &http, enabled: true }).unwrap();
    fixture.assert_import(&http, true);
    let document = fixture.document();
    let entry = &document["mcpServers"]["managed"];
    assert_eq!(entry["type"], "http");
    assert_eq!(entry["timeout"], 30);
    assert_eq!(entry["toolExposure"], json!({"write_*":"hidden"}));
    for key in ["command", "args", "env", "cwd"] {
        assert!(entry.get(key).is_none());
    }
    assert_eq!(document["enabledServers"], json!(["allow", "also-allow"]));

    fixture.apply(OmpMcpEdit::Disable).unwrap();
    fixture.assert_import(&http, false);
    let document = fixture.document();
    assert_eq!(document["mcpServers"]["managed"]["enabled"], false);
    assert_eq!(document["disabledServers"], json!(["deny", "managed"]));
    let disabled = fixture.bytes();
    fixture.apply(OmpMcpEdit::Disable).unwrap();
    assert_eq!(fixture.bytes(), disabled);

    let sse = McpConnectionDefinition::Sse {
        url: format!("http://{}/sse", listener.local_addr().unwrap()),
        headers: Default::default(),
    };
    fixture.apply(OmpMcpEdit::Update { connection: &sse, enabled: false }).unwrap();
    fixture.assert_import(&sse, false);
    assert_eq!(fixture.document()["disabledServers"], json!(["deny", "managed"]));
    fixture.apply(OmpMcpEdit::Update { connection: &sse, enabled: true }).unwrap();
    fixture.assert_import(&sse, true);
    let document = fixture.document();
    assert_eq!(document["disabledServers"], json!(["deny"]));
    assert_eq!(document["mcpServers"]["managed"]["type"], "sse");
    assert!(document["mcpServers"]["managed"].get("headers").is_none());
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);

    fixture.apply(OmpMcpEdit::Disable).unwrap();
    fixture.apply(OmpMcpEdit::Remove).unwrap();
    let removed = fixture.bytes();
    fixture.apply(OmpMcpEdit::Remove).unwrap();
    assert_eq!(fixture.bytes(), removed);
    assert_eq!(
        fixture.document(),
        json!({"autoEnableCodemode": false, "enabledServers": ["allow", "also-allow"],
            "disabledServers": ["deny"], "mcpServers": {"unmanaged": {"command":"keep"}}})
    );
}

#[test]
fn cc_mcp_t22_import_honors_denylist_and_reports_unsupported_transport() {
    let fixture = Fixture::new(
        r#"{
        "enabledServers": ["allow", "deny"],
        "disabledServers": ["deny"],
        "mcpServers": {
            "allow": {"type":"streamable-http","url":"http://127.0.0.1/mcp","enabled":"0"},
            "deny": {"command":"fixture","enabled":true},
            "off": {"command":"fixture","enabled":"FALSE"},
            "implicit-http": {"url":"http://127.0.0.1/mcp"},
            "unsupported": {"type":"websocket","url":"http://127.0.0.1/mcp"}
        }
    }"#,
    );
    let before = fixture.bytes();
    let entries = fixture.read();
    assert!(entries["allow"].as_ref().unwrap().enabled);
    assert!(!entries["deny"].as_ref().unwrap().enabled);
    assert!(!entries["off"].as_ref().unwrap().enabled);
    assert!(
        entries["allow"].as_ref().unwrap().connection
            == entries["implicit-http"].as_ref().unwrap().connection
    );
    assert!(matches!(entries["unsupported"], Err(OmpMcpError::UnsupportedTransport)));
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn cc_mcp_t22_missing_local_entry_can_be_denied_and_removed() {
    let fixture = Fixture::new("{}");
    let file = fixture.directory.join("mcp.json");
    fs::remove_file(&file).unwrap();
    fixture.apply(OmpMcpEdit::Remove).unwrap();
    assert!(!file.exists());
    fixture.apply(OmpMcpEdit::Disable).unwrap();
    assert_eq!(fixture.document(), json!({"disabledServers":["managed"]}));
    assert!(fixture.read().is_empty());
    fixture.apply(OmpMcpEdit::Remove).unwrap();
    assert_eq!(fixture.document(), json!({"disabledServers":[]}));
}

#[test]
fn cc_mcp_t22_conflicts_invalid_lists_and_unresolved_credentials_preserve_file() {
    let connection = McpConnectionDefinition::Stdio {
        command: "fixture".into(),
        args: vec![],
        env: Default::default(),
        cwd: None,
    };
    for original in [
        r#"{"mcpServers":{"managed":{"command":"external"}}}"#,
        r#"{"enabledServers":["managed"]}"#,
    ] {
        let fixture = Fixture::new(original);
        assert_eq!(
            fixture.apply(OmpMcpEdit::Insert(&connection)),
            Err(OmpMcpError::File(WriteError::Conflict))
        );
        assert_eq!(fixture.bytes(), original.as_bytes());
    }

    let malformed = Fixture::new(r#"{"disabledServers":"invalid"}"#);
    let before = malformed.bytes();
    assert_eq!(
        malformed.apply(OmpMcpEdit::Disable),
        Err(OmpMcpError::File(WriteError::InvalidResource))
    );
    assert_eq!(malformed.bytes(), before);

    let empty = Fixture::new("{}");
    let unresolved = McpConnectionDefinition::Sse {
        url: "http://127.0.0.1/sse".into(),
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
        empty.apply(OmpMcpEdit::Insert(&unresolved)),
        Err(OmpMcpError::CredentialUnavailable)
    );
    assert_eq!(empty.bytes(), b"{}");
}

#[test]
fn cc_mcp_t22_failed_preflight_publishes_neither_entry_nor_denylist() {
    let original = r#"{"mcpServers":{"managed":{"command":"fixture","enabled":true}},"disabledServers":["other"]}"#;
    let fixture = Fixture::new(original);
    let mut guard = lock_app(AppId::Omp, &fixture.directory).unwrap();
    let result = OmpMcp::new(&mut guard).unwrap().apply(
        &fixture.transactions,
        &fixture.tenant,
        "managed",
        OmpMcpEdit::Disable,
        || Err(WriteError::Conflict),
    );
    assert_eq!(result, Err(OmpMcpError::File(WriteError::Conflict)));
    assert_eq!(fixture.bytes(), original.as_bytes());
}
