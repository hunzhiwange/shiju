use super::mcode::{self, McodeMcpError, NativeServers, ServerEdit, ServerPatch};
use crate::providers::{
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::{AppWriteGuard, WriteError, lock_app},
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

const ID: &str = "managed";
const BEFORE: &[u8] =
    br#"{ "version": 7, "mcpServers": {"user": {"command":"keep","enabled":false}} }"#;

struct Fixture {
    home: tempfile::TempDir,
    path: PathBuf,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(before: Option<&[u8]>) -> Self {
        let home = tempfile::tempdir().unwrap();
        fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(home.path(), ExecutionHostId::new("fixture").unwrap())
            .unwrap();
        let settings = home.path().join("minimax-data");
        let environment = BTreeMap::from([
            ("MINIMAX_DATA_DIR".into(), settings.to_str().unwrap().into()),
            ("MAVIS_DATA_DIR".into(), home.path().join("unused").to_str().unwrap().into()),
        ]);
        let live = PathBuf::from(
            paths::read(home.path(), AppId::Mcode, None, &environment).unwrap().directory,
        );
        assert_eq!(live, settings);
        fs::create_dir(&live).unwrap();
        let path = live.join("mcp.json");
        if let Some(bytes) = before {
            fs::write(&path, bytes).unwrap();
        }
        Self { home, path, tx, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn guard(&self) -> AppWriteGuard {
        lock_app(AppId::Mcode, self.path.parent().unwrap()).unwrap()
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(&self.path).unwrap()
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.bytes()).unwrap()
    }

    fn read(&self) -> Result<NativeServers, WriteError> {
        mcode::read(&mut self.guard())
    }

    fn apply(&self, id: &str, edit: ServerEdit<'_>) -> Result<(), McodeMcpError> {
        self.apply_and_commit(id, edit, || Ok(()))
    }

    fn apply_and_commit(
        &self,
        id: &str,
        edit: ServerEdit<'_>,
        commit: impl FnOnce() -> Result<(), WriteError>,
    ) -> Result<(), McodeMcpError> {
        mcode::apply(
            &mut self.guard(),
            &self.tx,
            &self.tenant,
            ServerPatch { id, edit },
            || Ok(()),
            commit,
        )
    }

    fn assert_import(&self, connection: &McpConnectionDefinition) {
        let imported = self.read().unwrap();
        let server = imported[ID].as_ref().unwrap();
        assert!(&server.connection == connection);
        assert!(server.enabled);
    }
}

fn literal(name: &str, value: &str) -> BTreeMap<String, McpConfigValue> {
    BTreeMap::from([(name.into(), McpConfigValue::Literal { value: value.into() })])
}

fn stdio(marker: &std::path::Path) -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio {
        command: "/usr/bin/touch".into(),
        args: vec![marker.to_str().unwrap().into()],
        env: literal("FIXTURE", "harmless"),
        cwd: None,
    }
}

#[test]
fn cc_mcp_t18_projects_and_reads_lifecycle_without_starting_transports() {
    let fixture = Fixture::new(Some(BEFORE));
    let config_yaml = fixture.path.with_file_name("config.yaml");
    fs::write(&config_yaml, "model: fixture\n").unwrap();
    let marker = fixture.home.path().join("unused-marker");
    let command = stdio(&marker);
    fixture.apply(ID, ServerEdit::Insert(&command)).unwrap();
    assert!(fixture.json()["mcpServers"].get(ID).is_some(), "insert must project into mcp.json");
    fixture.assert_import(&command);

    let mut document = fixture.json();
    document["mcpServers"][ID]["timeout"] = json!(5000);
    fs::write(&fixture.path, document.to_string()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = listener.local_addr().unwrap();
    let http = McpConnectionDefinition::StreamableHttp {
        url: format!("http://{endpoint}/mcp"),
        headers: literal("X-Fixture", "http"),
    };
    fixture.apply(ID, ServerEdit::Update(&http)).unwrap();
    assert_eq!(
        fixture.json()["mcpServers"][ID],
        json!({
            "type": "http", "url": format!("http://{endpoint}/mcp"),
            "headers": {"X-Fixture":"http"}, "timeout": 5000, "enabled": true
        })
    );
    fixture.assert_import(&http);
    let sse = McpConnectionDefinition::Sse {
        url: format!("http://{endpoint}/events"),
        headers: literal("X-Fixture", "sse"),
    };
    fixture.apply(ID, ServerEdit::Update(&sse)).unwrap();
    fixture.assert_import(&sse);
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    assert!(!marker.exists());
    assert!(!fixture.read().unwrap()["user"].as_ref().unwrap().enabled);

    fixture.apply(ID, ServerEdit::Disable).unwrap();
    let disabled = fixture.bytes();
    assert!(!fixture.read().unwrap().contains_key(ID));
    fixture.apply(ID, ServerEdit::Remove).unwrap();
    assert_eq!(fixture.bytes(), disabled);
    assert_eq!(fixture.json(), serde_json::from_slice::<Value>(BEFORE).unwrap());
    assert_eq!(fs::read_to_string(config_yaml).unwrap(), "model: fixture\n");
    assert_eq!(fs::metadata(&fixture.path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn cc_mcp_t18_import_distinguishes_unsupported_transports_and_valid_neighbors() {
    let fixture = Fixture::new(Some(
        br#"{"mcpServers":{
        "unsupported":{"type":"websocket","url":"http://127.0.0.1/mcp"},
        "alias":{"type":"streamable-http","url":"http://127.0.0.1/mcp","timeout":5000},
        "implicit":{"url":"http://127.0.0.1/mcp","enabled":false},
        "ambiguous":{"command":"node","url":"http://127.0.0.1/mcp"},
        "broken":{"command":"node","enabled":"false"}
    }}"#,
    ));
    let before = fixture.bytes();
    let servers = fixture.read().unwrap();
    assert!(matches!(servers.get("unsupported"), Some(Err(McodeMcpError::UnsupportedTransport))));
    assert!(matches!(servers["ambiguous"], Err(McodeMcpError::InvalidDefinition)));
    assert!(matches!(servers["broken"], Err(McodeMcpError::InvalidDefinition)));
    let alias = servers["alias"].as_ref().unwrap();
    let implicit = servers["implicit"].as_ref().unwrap();
    assert!(matches!(alias.connection, McpConnectionDefinition::StreamableHttp { .. }));
    assert!(alias.connection == implicit.connection);
    assert!(alias.enabled);
    assert!(!implicit.enabled);
    assert_eq!(fixture.bytes(), before);
}

#[test]
fn cc_mcp_t18_rejects_collisions_and_invalid_file_without_committing() {
    let fixture = Fixture::new(Some(BEFORE));
    let connection = stdio(&fixture.home.path().join("marker"));
    assert_eq!(
        fixture.apply("user", ServerEdit::Insert(&connection)),
        Err(McodeMcpError::File(WriteError::Conflict)),
    );
    assert_eq!(fixture.bytes(), BEFORE);
    let invalid = br#"{"mcpServers":[]}"#;
    fs::write(&fixture.path, invalid).unwrap();
    assert_eq!(
        fixture.apply(ID, ServerEdit::Insert(&connection)),
        Err(McodeMcpError::File(WriteError::InvalidResource)),
    );
    assert_eq!(fixture.bytes(), invalid);
}

#[test]
fn cc_mcp_t18_failed_catalog_commit_restores_existing_and_missing_native_file() {
    for previous in [None, Some(BEFORE)] {
        let fixture = Fixture::new(previous);
        let connection = stdio(&fixture.home.path().join("marker"));
        let catalog = fixture.home.path().join("catalog");
        fs::create_dir(&catalog).unwrap();
        let result = fixture.apply_and_commit(ID, ServerEdit::Insert(&connection), || {
            assert!(fixture.path.is_file(), "native write must precede catalog commit");
            fs::write(&catalog, "saved").map_err(|_| WriteError::Io)
        });
        assert_eq!(result, Err(McodeMcpError::Catalog(WriteError::Io)));
        assert_eq!(fs::read(&fixture.path).ok().as_deref(), previous);
        assert!(fixture.tx.pending(AppId::Mcode).unwrap().is_none());
        fs::remove_dir(&catalog).unwrap();
        fixture
            .apply_and_commit(ID, ServerEdit::Insert(&connection), || {
                fs::write(&catalog, "saved").map_err(|_| WriteError::Io)
            })
            .unwrap();
        fixture.assert_import(&connection);
        assert_eq!(fs::read_to_string(catalog).unwrap(), "saved");
    }
}

#[test]
fn cc_mcp_t18_rollback_conflict_preserves_external_config() {
    let fixture = Fixture::new(Some(BEFORE));
    let connection = stdio(&fixture.home.path().join("marker"));
    let result = fixture.apply_and_commit(ID, ServerEdit::Insert(&connection), || {
        assert!(fixture.json()["mcpServers"].get(ID).is_some());
        fs::write(&fixture.path, br#"{"external":true}"#).unwrap();
        Err(WriteError::Conflict)
    });
    assert_eq!(
        result,
        Err(McodeMcpError::Rollback { catalog: WriteError::Conflict, file: WriteError::Conflict })
    );
    assert_eq!(fixture.json(), json!({"external":true}));
    assert!(fixture.tx.pending(AppId::Mcode).unwrap().is_none());
}

#[test]
fn cc_mcp_t18_invalid_projection_never_calls_catalog() {
    let fixture = Fixture::new(Some(BEFORE));
    let with_cwd = McpConnectionDefinition::Stdio {
        command: "node".into(),
        args: vec![],
        env: BTreeMap::new(),
        cwd: Some(fixture.home.path().to_str().unwrap().into()),
    };
    let unresolved = McpConnectionDefinition::StreamableHttp {
        url: "http://127.0.0.1/mcp".into(),
        headers: BTreeMap::from([(
            "Authorization".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::CodexOauth,
                    account_id: "fixture".into(),
                },
            },
        )]),
    };
    for (connection, error) in [
        (with_cwd, McodeMcpError::UnsupportedField("cwd")),
        (unresolved, McodeMcpError::CredentialUnavailable),
    ] {
        let result = fixture.apply_and_commit(ID, ServerEdit::Insert(&connection), || {
            panic!("invalid projection must not commit")
        });
        assert_eq!(result, Err(error));
        assert_eq!(fixture.bytes(), BEFORE);
    }
}

#[test]
fn cc_mcp_t18_stale_binding_skips_publication_and_catalog_commit() {
    let fixture = Fixture::new(Some(BEFORE));
    let connection = stdio(&fixture.home.path().join("marker"));
    let result = mcode::apply(
        &mut fixture.guard(),
        &fixture.tx,
        &fixture.tenant,
        ServerPatch { id: ID, edit: ServerEdit::Insert(&connection) },
        || Err(WriteError::Conflict),
        || panic!("stale revision must not commit"),
    );
    assert_eq!(result, Err(McodeMcpError::File(WriteError::Conflict)));
    assert_eq!(fixture.bytes(), BEFORE);
    assert!(fixture.tx.pending(AppId::Mcode).unwrap().is_none());
}
