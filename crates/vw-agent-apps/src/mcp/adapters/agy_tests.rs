use super::agy::{AgyMcp, AgyMcpEdit, AgyMcpError};
use crate::providers::live::{
    commit::{ExecutionHostId, ProviderTransactions},
    write_guard::WriteError,
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{
    TenantScope,
    mcp_management::{McpConfigValue::Literal, McpConnectionDefinition},
};

const EXISTING: &str = r#"{"extra":1,"mcpServers":{"user":{"command":"keep"}}}"#;
const HTTP_URL: &str = "http://127.0.0.1:49152/mcp";

struct Fixture {
    home: tempfile::TempDir,
    _store: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let home = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        fs::set_permissions(store.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let transactions =
            ProviderTransactions::new(store.path(), ExecutionHostId::new("test-host").unwrap())
                .unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let fixture = Self { home, _store: store, transactions, tenant };
        let path = fixture.path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
        fixture
    }

    fn path(&self) -> PathBuf {
        self.home.path().join(".gemini/config/mcp_config.json")
    }

    fn adapter(&self) -> AgyMcp {
        AgyMcp::open(self.home.path()).unwrap()
    }

    fn apply(&self, id: &str, edit: AgyMcpEdit<'_>) -> Result<(), AgyMcpError> {
        self.adapter().apply(&self.transactions, &self.tenant, id, edit, || Ok(()))
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&fs::read(self.path()).unwrap()).unwrap()
    }

    fn check_import(&self, id: &str, connection: &McpConnectionDefinition, enabled: bool) {
        let imported = self.adapter().read().unwrap();
        let server = imported.get(id).expect("the written server must import").as_ref().unwrap();
        assert!(server.connection == *connection, "the imported transport must be equivalent");
        assert_eq!(server.enabled, enabled);
    }
}

fn stdio() -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio {
        command: "mcp-fixture".into(),
        args: vec!["--serve".into(), " keep spaces ".into()],
        env: [("FIXTURE_MODE".into(), Literal { value: "test".into() })].into(),
        cwd: Some("/fixture".into()),
    }
}

fn http() -> McpConnectionDefinition {
    McpConnectionDefinition::StreamableHttp {
        url: HTTP_URL.into(),
        headers: [("X-Mode".into(), Literal { value: "test".into() })].into(),
    }
}

#[test]
fn cc_mcp_t21_agy_projects_and_reads_managed_lifecycle() {
    let fixture = Fixture::new(EXISTING.as_bytes());
    let id = "managed";
    let stdio = stdio();
    fixture.apply(id, AgyMcpEdit::Insert(&stdio)).expect("AGY must publish the stdio binding");
    fixture.check_import(id, &stdio, true);
    assert_eq!(
        fixture.apply("user", AgyMcpEdit::Insert(&stdio)),
        Err(AgyMcpError::File(WriteError::Conflict))
    );

    let mut document = fixture.json();
    document["mcpServers"][id]["timeout"] = json!(3000);
    fs::write(fixture.path(), serde_json::to_vec(&document).unwrap()).unwrap();

    let http = http();
    fixture.apply(id, AgyMcpEdit::Update(&http)).unwrap();
    let after = fixture.json();
    assert_eq!(
        after["mcpServers"][id],
        json!({"serverUrl": HTTP_URL, "headers": {"X-Mode": "test"}, "timeout": 3000, "disabled": false})
    );
    fixture.check_import(id, &http, true);
    assert_eq!(fs::metadata(fixture.path()).unwrap().permissions().mode() & 0o777, 0o600);

    fixture.apply(id, AgyMcpEdit::Disable).unwrap();
    fixture.check_import(id, &http, false);
    assert_eq!(fixture.json()["mcpServers"][id]["timeout"], 3000);
    assert_eq!(fixture.json()["mcpServers"][id]["disabled"], true);

    fixture.apply(id, AgyMcpEdit::Remove).unwrap();
    assert!(!fixture.adapter().read().unwrap().contains_key(id));
    assert_eq!(fixture.json(), serde_json::from_str::<Value>(EXISTING).unwrap());
}

#[test]
fn cc_mcp_t21_agy_rejects_sse_and_stale_preflight_without_writing() {
    let fixture = Fixture::new(EXISTING.as_bytes());
    let sse = McpConnectionDefinition::Sse {
        url: "http://127.0.0.1:49152/events".into(),
        headers: Default::default(),
    };
    assert_eq!(
        fixture.apply("managed", AgyMcpEdit::Insert(&sse)),
        Err(AgyMcpError::UnsupportedTransport)
    );
    assert_eq!(
        fixture.adapter().apply(
            &fixture.transactions,
            &fixture.tenant,
            "managed",
            AgyMcpEdit::Insert(&http()),
            || Err(WriteError::Conflict)
        ),
        Err(AgyMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fs::read(fixture.path()).unwrap(), EXISTING.as_bytes());
}

#[test]
fn cc_mcp_t21_agy_handles_empty_file_and_rejects_corrupt_config() {
    for bytes in [b"".as_slice(), b" \n\t "] {
        let fixture = Fixture::new(bytes);
        assert!(fixture.adapter().read().unwrap().is_empty());
        fixture.apply("missing", AgyMcpEdit::Disable).unwrap();
        fixture.apply("missing", AgyMcpEdit::Remove).unwrap();
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
        fixture.apply("managed", AgyMcpEdit::Insert(&stdio())).unwrap();
        fixture.check_import("managed", &stdio(), true);
    }
    for bytes in [b"{broken".as_slice(), br#"{"mcpServers":[]}"#] {
        let fixture = Fixture::new(bytes);
        assert_eq!(fixture.adapter().read().err(), Some(WriteError::InvalidResource));
        assert_eq!(
            fixture.apply("managed", AgyMcpEdit::Insert(&stdio())),
            Err(AgyMcpError::File(WriteError::InvalidResource))
        );
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
    }
}

#[test]
fn cc_mcp_t21_agy_imports_legacy_url_and_reports_unsupported_entries() {
    let fixture = Fixture::new(
        br#"{"mcpServers":{
            "legacy":{"url":"http://127.0.0.1:49152/mcp","type":"streamable-http",
                "headers":{"X-Mode":"test"},"disabled":true,"timeout":3000},
            "events":{"type":"sse","url":"http://127.0.0.1:49152/events"},
            "bad":{"command":false}
        }}"#,
    );
    fixture.check_import("legacy", &http(), false);
    let imported = fixture.adapter().read().unwrap();
    assert_eq!(imported["events"].as_ref().err(), Some(&AgyMcpError::UnsupportedTransport));
    assert_eq!(imported["bad"].as_ref().err(), Some(&AgyMcpError::InvalidDefinition));
}
