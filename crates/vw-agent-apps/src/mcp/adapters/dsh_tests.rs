use super::{DshError, NativeServers, ServerEdit, read_servers, sync_and_commit};
use crate::{
    mcp::validation::parse_native,
    providers::{
        dsh::{DshFiles, Snapshot},
        live::{
            commit::{ExecutionHostId, ProviderTransactions},
            write_guard::WriteError,
        },
    },
};
use serde_json::json;
use serde_yaml::Value as Yaml;
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt};
use vw_api_types::{TenantScope, agent_apps::AppId};

const BEFORE: &str = "# fixture\n- id: custom\n  config: {dynamic: !!js process.env.VALUE}\n";
const PROFILES: [&str; 3] = ["web", "desktop", "headless"];

struct Fixture {
    home: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(profile: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        for name in PROFILES {
            let directory = home.path().join(".dsh/profiles").join(name);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("cordis.patch.yml"), profile).unwrap();
        }
        let state = home.path().join("transactions");
        fs::create_dir(&state).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
        let transactions =
            ProviderTransactions::new(&state, ExecutionHostId::new("test-host").unwrap()).unwrap();
        Self { home, transactions, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn edit(&self, id: &str, edit: ServerEdit<'_>) -> Result<(), DshError> {
        self.edit_with_commit(id, edit, || Ok(()))
    }

    fn edit_with_commit(
        &self,
        id: &str,
        edit: ServerEdit<'_>,
        commit: impl FnOnce() -> Result<(), WriteError>,
    ) -> Result<(), DshError> {
        let mut files = DshFiles::open(self.home.path()).unwrap();
        sync_and_commit(&mut files, &self.transactions, &self.tenant, id, edit, || Ok(()), commit)
    }

    fn snapshot(&self) -> Snapshot {
        crate::providers::dsh::read(self.home.path()).unwrap()
    }

    fn imported(&self) -> NativeServers {
        read_servers(&self.snapshot()).unwrap()
    }
}

#[test]
fn cc_mcp_t20_projects_and_reverses_managed_dsh_lifecycle() {
    let fixture = Fixture::new(BEFORE);
    let stdio = parse_native(json!({
        "command": "/usr/bin/printf", "args": ["fixture"], "env": {"LANG": "C"}, "cwd": "/tmp"
    }))
    .unwrap();
    fixture.edit("managed", ServerEdit::Insert(&stdio)).unwrap();
    let imported = fixture.imported();
    assert!(imported.contains_key("managed"), "insert must publish the DSH server");
    let server = imported["managed"].as_ref().unwrap();
    assert!(server.enabled && server.connection == stdio);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let http = parse_native(json!({
        "url": format!("http://{}/mcp", listener.local_addr().unwrap()),
        "headers": {"X-Test": "fixture"}
    }))
    .unwrap();
    fixture.edit("managed", ServerEdit::Update(&http)).unwrap();
    let snapshot = fixture.snapshot();
    assert!(!snapshot.contains_key("acp"));
    for profile in PROFILES {
        let bytes = snapshot[profile].as_deref().unwrap();
        assert!(bytes.starts_with(BEFORE.as_bytes()));
        let yaml: Yaml = serde_yaml::from_slice(bytes).unwrap();
        let plugin = &yaml[1]["insert"][0];
        assert_eq!(plugin["name"].as_str(), Some("@deepseek-ai/dsh-mcp-client"));
        assert_eq!(plugin["config"]["serverName"].as_str(), Some("managed"));
        assert_eq!(plugin["config"]["transport"].as_str(), Some("streamable-http"));
        assert_eq!(plugin["config"]["headers"]["X-Test"].as_str(), Some("fixture"));
        assert!(plugin["config"].get("command").is_none());
    }

    fixture.edit("managed", ServerEdit::Disable).unwrap();
    let imported = fixture.imported();
    let server = imported["managed"].as_ref().unwrap();
    assert!(!server.enabled && server.connection == http);
    fixture.edit("managed", ServerEdit::Update(&http)).unwrap();
    assert!(fixture.imported()["managed"].as_ref().unwrap().enabled);
    fixture.edit("managed", ServerEdit::Remove).unwrap();
    fixture.edit("managed", ServerEdit::Remove).unwrap();
    assert!(!fixture.imported().contains_key("managed"));
    for profile in PROFILES {
        assert_eq!(fixture.snapshot()[profile].as_deref(), Some(BEFORE.as_bytes()));
    }
}

#[test]
fn cc_mcp_t20_restores_every_profile_after_catalog_commit_fails() {
    let fixture = Fixture::new(BEFORE);
    fs::remove_file(fixture.home.path().join(".dsh/profiles/desktop/cordis.patch.yml")).unwrap();
    let original = fixture.snapshot();
    let connection = parse_native(json!({"command": "/usr/bin/printf"})).unwrap();
    let result = fixture.edit_with_commit("managed", ServerEdit::Insert(&connection), || {
        assert!(fixture.imported().contains_key("managed"), "catalog commit follows publication");
        Err(WriteError::Io)
    });
    assert_eq!(result, Err(DshError::Catalog(WriteError::Io)));
    assert_eq!(fixture.snapshot(), original);
    assert!(fixture.transactions.pending(AppId::Dsh).unwrap().is_none());
}

#[test]
fn cc_mcp_t20_rejects_sse_without_touching_profiles() {
    let fixture = Fixture::new(BEFORE);
    let original = fixture.snapshot();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let sse = parse_native(json!({
        "type": "sse", "url": format!("http://{}/events", listener.local_addr().unwrap())
    }))
    .unwrap();
    assert_eq!(
        fixture.edit("managed", ServerEdit::Insert(&sse)),
        Err(DshError::UnsupportedTransport)
    );
    assert_eq!(fixture.snapshot(), original);
}

#[test]
fn cc_mcp_t20_updates_a_native_group_without_losing_options_or_siblings() {
    let fixture = Fixture::new(
        "- insert:\n  - id: native-mcp\n    name: '@deepseek-ai/dsh-mcp-client'\n    disabled: true\n    config: {serverName: managed, transport: stdio, command: old, toolCallTimeoutMs: 5000}\n  - id: custom\n    config: {keep: true}\n",
    );
    let connection = parse_native(json!({"command": "new"})).unwrap();
    let before = fixture.snapshot();
    assert_eq!(
        fixture.edit("managed", ServerEdit::Insert(&connection)),
        Err(DshError::Native(WriteError::Conflict))
    );
    assert_eq!(fixture.snapshot(), before);
    fixture.edit("managed", ServerEdit::Update(&connection)).unwrap();
    let imported = fixture.imported();
    let server = imported["managed"].as_ref().unwrap();
    assert!(server.enabled && server.connection == connection);

    let after = fixture.snapshot();
    let yaml: Yaml = serde_yaml::from_slice(after["web"].as_deref().unwrap()).unwrap();
    assert_eq!(yaml[0]["insert"][0]["id"].as_str(), Some("native-mcp"));
    assert_eq!(yaml[0]["insert"][0]["config"]["toolCallTimeoutMs"].as_u64(), Some(5000));
    assert_eq!(yaml[0]["insert"][1]["config"]["keep"].as_bool(), Some(true));
    fixture.edit("managed", ServerEdit::Remove).unwrap();
    let after = fixture.snapshot();
    let yaml: Yaml = serde_yaml::from_slice(after["web"].as_deref().unwrap()).unwrap();
    assert_eq!(yaml[0]["insert"][0]["id"].as_str(), Some("custom"));
}

#[test]
fn cc_mcp_t20_rejects_broken_yaml_and_divergent_profile_edits() {
    let fixture = Fixture::new(BEFORE);
    let path = fixture.home.path().join(".dsh/profiles/headless/cordis.patch.yml");
    fs::write(&path, "- insert: [broken\n").unwrap();
    let original = fixture.snapshot();
    let connection = parse_native(json!({"command": "new"})).unwrap();
    assert_eq!(
        fixture.edit("managed", ServerEdit::Insert(&connection)),
        Err(DshError::Native(WriteError::InvalidResource))
    );
    assert_eq!(fixture.snapshot(), original);

    fs::write(&path, BEFORE).unwrap();
    fixture.edit("managed", ServerEdit::Insert(&connection)).unwrap();
    let external = fs::read_to_string(&path).unwrap().replace("command: new", "command: external");
    fs::write(&path, external).unwrap();
    let original = fixture.snapshot();
    assert_eq!(
        fixture.edit("managed", ServerEdit::Remove),
        Err(DshError::Native(WriteError::Conflict))
    );
    assert_eq!(
        fixture.imported()["managed"].as_ref().err(),
        Some(&DshError::Native(WriteError::Conflict))
    );
    assert_eq!(fixture.snapshot(), original);
}

#[test]
fn cc_mcp_t20_rejects_unresolved_credentials_and_imports_unsupported_transport_as_error() {
    let fixture = Fixture::new(
        "- id: native-mcp\n  name: '@deepseek-ai/dsh-mcp-client'\n  config: {serverName: native, transport: sse, url: 'http://localhost/events'}\n",
    );
    assert_eq!(fixture.imported()["native"].as_ref().err(), Some(&DshError::UnsupportedTransport));
    let connection = serde_json::from_value(json!({
        "transport": "stdio",
        "command": "mcp-fixture",
        "env": {"TOKEN": {"kind": "credential", "reference": {"service": "codex_oauth", "accountId": "fixture"}}}
    }))
    .unwrap();
    let original = fixture.snapshot();
    assert_eq!(
        fixture.edit("managed", ServerEdit::Insert(&connection)),
        Err(DshError::UnresolvedCredential)
    );
    assert_eq!(fixture.snapshot(), original);
}
