use super::hermes::{self, HermesError, NativeServers, ServerEdit, ServerPatch};
use crate::providers::{
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::{WriteError, lock_app},
    },
    paths,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, net::TcpListener, path::Path};
use vw_api_types::{agent_apps::AppId, id::TenantScope, mcp_management::McpConnectionDefinition};

const BEFORE: &[u8] = b"# root comment\nmodel: 'keep'\nmcp_servers: # services\n  user: {command: 'keep'} # untouched\n";

struct Fixture {
    home: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(before: &[u8]) -> Self {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join(".hermes")).unwrap();
        fs::write(home.path().join(".hermes/config.yaml"), before).unwrap();
        let state = home.path().join("transactions");
        fs::create_dir(&state).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
        let transactions =
            ProviderTransactions::new(&state, ExecutionHostId::new("host").unwrap()).unwrap();
        Self { home, transactions, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn snapshot(&self) -> Vec<u8> {
        fs::read(self.home.path().join(".hermes/config.yaml")).unwrap()
    }

    fn publish(&self, id: &str, edit: ServerEdit<'_>) -> Result<(), WriteError> {
        let paths = paths::read(self.home.path(), AppId::Hermes, None, &BTreeMap::new()).unwrap();
        let mut guard = lock_app(AppId::Hermes, Path::new(&paths.directory)).unwrap();
        let file = hermes::file(&mut guard).unwrap();
        let before = file.read()?;
        let patch = ServerPatch::new(id, edit).unwrap();
        file.commit(&self.transactions, &self.tenant, before.as_deref(), &patch, || Ok(()))
    }

    fn imported(&self) -> NativeServers {
        hermes::read_servers(Some(&self.snapshot())).unwrap()
    }
}

fn definition(value: Value) -> McpConnectionDefinition {
    crate::mcp::validation::parse_native(value).unwrap()
}

#[test]
fn cc_mcp_t16_projects_and_reads_managed_hermes_lifecycle() {
    let fixture = Fixture::new(BEFORE);
    let stdio = definition(json!({
        "command": "/usr/bin/printf",
        "args": ["hello"],
        "env": {"LANG": "C"}
    }));
    fixture.publish("managed", ServerEdit::Insert(&stdio)).unwrap();
    let imported = fixture.imported();
    assert!(imported.contains_key("managed"), "insert must publish the Hermes server");
    let server = imported["managed"].as_ref().unwrap();
    assert!(server.connection == stdio && server.enabled);

    let config = fixture.home.path().join(".hermes/config.yaml");
    let mut customized = fixture.snapshot();
    customized.extend_from_slice(b"    timeout: 45\n    tools: {include: [read]}\n    future_option: [1, true]\n    auth: oauth\n");
    fs::write(config, customized).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let http = definition(json!({
        "url": format!("http://{}/mcp", listener.local_addr().unwrap()),
        "headers": {"X-Test": "fixture"}
    }));
    let sse = definition(json!({
        "type": "sse",
        "url": format!("http://{}/events", listener.local_addr().unwrap()),
        "headers": {"X-Test": "fixture"}
    }));
    for connection in [&sse, &http] {
        fixture.publish("managed", ServerEdit::Update(connection)).unwrap();
        let imported = fixture.imported();
        let server = imported["managed"].as_ref().unwrap();
        assert!(&server.connection == connection && server.enabled);
        let yaml: serde_yaml::Value = serde_yaml::from_slice(&fixture.snapshot()).unwrap();
        let node = &yaml["mcp_servers"]["managed"];
        for stale in ["command", "args", "env"] {
            assert!(node.get(stale).is_none());
        }
        assert_eq!(node["timeout"].as_u64(), Some(45));
        assert_eq!(node["auth"].as_str(), Some("oauth"));
        assert_eq!(node["tools"]["include"][0].as_str(), Some("read"));
        assert_eq!(node["future_option"][1].as_bool(), Some(true));
        assert_eq!(
            node.get("transport").and_then(serde_yaml::Value::as_str),
            (connection == &sse).then_some("sse")
        );
    }

    fixture.publish("managed", ServerEdit::Disable).unwrap();
    let imported = fixture.imported();
    let server = imported["managed"].as_ref().unwrap();
    assert!(!server.enabled && server.connection == http);
    fixture.publish("managed", ServerEdit::Update(&sse)).unwrap();
    let imported = fixture.imported();
    let server = imported["managed"].as_ref().unwrap();
    assert!(!server.enabled && server.connection == sse);
    fixture.publish("managed", ServerEdit::Remove).unwrap();
    fixture.publish("managed", ServerEdit::Remove).unwrap();
    assert!(!fixture.imported().contains_key("managed"));
    assert_eq!(fixture.snapshot(), BEFORE);
    assert!(fixture.transactions.pending(AppId::Hermes).unwrap().is_none());
}

#[test]
fn cc_mcp_t16_rejects_unmanaged_collision_and_keeps_original_bytes() {
    let fixture = Fixture::new(BEFORE);
    let connection = definition(json!({"command": "different"}));
    assert_eq!(fixture.publish("user", ServerEdit::Insert(&connection)), Err(WriteError::Conflict));
    assert_eq!(fixture.snapshot(), BEFORE);
}

#[test]
fn cc_mcp_t16_reports_unsupported_transport_without_losing_valid_imports() {
    let fixture = Fixture::new(b"mcp_servers:\n  invalid: {url: 'https://example.test/mcp', transport: grpc}\n  good: {command: sample}\n");
    let servers = fixture.imported();
    assert_eq!(
        servers.get("invalid").and_then(|server| server.as_ref().err()),
        Some(&HermesError::UnsupportedTransport)
    );
    assert!(servers["good"].as_ref().unwrap().enabled);
    let cwd = definition(json!({"command": "sample", "cwd": "/somewhere"}));
    assert_eq!(
        ServerPatch::new("managed", ServerEdit::Insert(&cwd)).err(),
        Some(HermesError::UnsupportedField("cwd"))
    );
    let unresolved = serde_json::from_value(json!({
        "transport": "stdio",
        "command": "sample",
        "env": {"TOKEN": {"kind": "credential", "reference": {"service": "codex_oauth", "accountId": "fixture"}}}
    })).unwrap();
    assert_eq!(
        ServerPatch::new("managed", ServerEdit::Insert(&unresolved)).err(),
        Some(HermesError::UnresolvedCredential)
    );
}

#[test]
fn cc_mcp_t16_accepts_empty_server_map_and_rejects_broken_yaml() {
    let fixture = Fixture::new(b"mcp_servers: {} # keep\nmodel: default\n");
    let connection = definition(json!({"command": "sample"}));
    fixture.publish("managed", ServerEdit::Insert(&connection)).unwrap();
    assert!(fixture.imported().contains_key("managed"));
    let after = fixture.snapshot();
    assert!(after.starts_with(b"mcp_servers:  # keep\n"));
    assert!(after.ends_with(b"model: default\n"));
    let before = b"mcp_servers: [broken\n";
    let fixture = Fixture::new(before);
    assert_eq!(
        fixture.publish("managed", ServerEdit::Insert(&connection)),
        Err(WriteError::InvalidResource)
    );
    assert_eq!(fixture.snapshot(), before);
}
