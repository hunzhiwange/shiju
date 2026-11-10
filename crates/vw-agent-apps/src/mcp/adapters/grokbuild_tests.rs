use super::grokbuild::{self, GrokMcpEdit, GrokMcpError};
use crate::providers::{
    grok::GrokFiles,
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{TenantScope, mcp_management::McpConnectionDefinition};

const EXISTING: &str = r#"# user configuration
[models]
default = "grok" # keep
[mcp_servers.user]
command = "keep"
"#;

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
        self.home.path().join(".grok/config.toml")
    }

    fn apply(&self, id: &str, edit: GrokMcpEdit<'_>) -> Result<(), GrokMcpError> {
        let mut files = GrokFiles::open(self.home.path()).unwrap();
        grokbuild::apply(&mut files.guard, &self.tx, &self.tenant, id, edit, || Ok(()))
    }

    fn check_import(&self, id: &str, expected: Option<&McpConnectionDefinition>) {
        let mut files = GrokFiles::open(self.home.path()).unwrap();
        let imported = grokbuild::read(&mut files.guard).unwrap();
        let connection = imported.get(id).map(|result| result.as_ref().unwrap());
        assert!(connection == expected, "native import must match the projected connection");
    }
}

#[test]
fn cc_mcp_t14_grokbuild_projects_and_reverses_managed_lifecycle() {
    use vw_api_types::mcp_management::McpConfigValue::Literal;

    let fixture = Fixture::new(Some(EXISTING));
    let id = "managed.tools";
    let stdio = McpConnectionDefinition::Stdio {
        command: "mcp-test".into(),
        args: vec!["--serve".into(), " keep spaces ".into()],
        env: [("MCP_MODE".into(), Literal { value: "fixture".into() })].into(),
        cwd: Some("/fixture".into()),
    };
    fixture
        .apply(id, GrokMcpEdit::Insert(&stdio))
        .expect("Grok Build must publish the stdio binding");
    fixture.check_import(id, Some(&stdio));
    assert_eq!(
        fixture.apply("user", GrokMcpEdit::Insert(&stdio)),
        Err(GrokMcpError::Native(WriteError::Conflict))
    );

    let http = McpConnectionDefinition::StreamableHttp {
        url: "http://127.0.0.1:49152/mcp".into(),
        headers: [("X-Mode".into(), Literal { value: "fixture".into() })].into(),
    };
    fixture.apply(id, GrokMcpEdit::Update(&http)).unwrap();
    fixture.check_import(id, Some(&http));
    let after = fs::read_to_string(fixture.path()).unwrap();
    let doc: toml::Table = after.parse().unwrap();
    let server = doc["mcp_servers"][id].as_table().unwrap();
    assert_eq!(server["headers"]["X-Mode"].as_str(), Some("fixture"));
    assert_eq!(server.keys().count(), 2, "the HTTP projection must contain only url and headers");
    assert_eq!(fs::metadata(fixture.path()).unwrap().permissions().mode() & 0o777, 0o600);

    fixture.apply(id, GrokMcpEdit::Remove).unwrap(); // disable
    fixture.check_import(id, None);
    fixture.apply(id, GrokMcpEdit::Remove).unwrap(); // delete after disable
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), EXISTING);
}

#[test]
fn cc_mcp_t14_grokbuild_imports_legacy_headers_and_isolates_invalid_entries() {
    let fixture = Fixture::new(Some(
        r#"[mcp_servers]
good = { url = "https://example.invalid/mcp", http_headers = { "X-Mode" = "fixture" } }
bad = { command = false }
legacy_sse = { type = "sse", url = "https://example.invalid/events" }
"#,
    ));
    let mut files = GrokFiles::open(fixture.home.path()).unwrap();
    let imported = grokbuild::read(&mut files.guard).unwrap();
    assert_eq!(imported["bad"].as_ref().err(), Some(&GrokMcpError::InvalidDefinition));
    assert_eq!(imported["legacy_sse"].as_ref().err(), Some(&GrokMcpError::UnsupportedTransport));
    let McpConnectionDefinition::StreamableHttp { headers, .. } =
        imported["good"].as_ref().unwrap()
    else {
        panic!("a URL with no type must import as Streamable HTTP");
    };
    assert!(matches!(
        headers.get("X-Mode"),
        Some(vw_api_types::mcp_management::McpConfigValue::Literal { value }) if value == "fixture"
    ));
    drop(files);

    // Revocation also works when the existing map is an inline table.
    let fixture = Fixture::new(Some(r#"mcp_servers = { user = { command = "keep" } }"#));
    fixture.apply("user", GrokMcpEdit::Remove).unwrap();
    fixture.check_import("user", None);
}

#[test]
fn cc_mcp_t14_grokbuild_rejects_lossy_transport_and_unresolved_values() {
    use vw_api_types::{
        agent_apps_auth::AuthService,
        mcp_management::{McpConfigValue, McpCredentialRef},
    };

    let fixture = Fixture::new(Some(EXISTING));
    let sse = McpConnectionDefinition::Sse {
        url: "http://127.0.0.1:49152/events".into(),
        headers: Default::default(),
    };
    assert_eq!(
        fixture.apply("managed", GrokMcpEdit::Insert(&sse)),
        Err(GrokMcpError::UnsupportedTransport)
    );

    let http = McpConnectionDefinition::StreamableHttp {
        url: "http://127.0.0.1:49152/mcp".into(),
        headers: [(
            "Authorization".into(),
            McpConfigValue::Credential {
                reference: McpCredentialRef {
                    service: AuthService::XaiOauth,
                    account_id: "fixture".into(),
                },
            },
        )]
        .into(),
    };
    assert_eq!(
        fixture.apply("managed", GrokMcpEdit::Insert(&http)),
        Err(GrokMcpError::UnresolvedCredential)
    );
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), EXISTING);
}

#[test]
fn cc_mcp_t14_grokbuild_preserves_corrupt_documents_and_missing_files() {
    for invalid in ["[mcp_servers.", "mcp_servers = []"] {
        let fixture = Fixture::new(Some(invalid));
        assert_eq!(
            fixture.apply("managed", GrokMcpEdit::Remove),
            Err(GrokMcpError::Native(WriteError::InvalidResource))
        );
        assert_eq!(fs::read_to_string(fixture.path()).unwrap(), invalid);
    }

    let fixture = Fixture::new(None);
    fixture.apply("managed", GrokMcpEdit::Remove).unwrap();
    assert!(!fixture.path().exists());
    fixture.check_import("managed", None);
}
