use super::claude::{ClaudeMcpEdit, ClaudeMcpError, apply, read};
use crate::{
    auth::{
        credential_file::{AccountCredential, CredentialFileStore},
        credential_lease::CredentialLease,
    },
    mcp::validation::parse_native,
    providers::live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
};
use serde_json::{Value, json};
use std::{
    fs,
    net::TcpListener,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
};
use vw_api_types::{TenantScope, agent_apps::AppId};

const BEFORE: &str = r#"{
  "projects": {"/fixture": {"mcpServers":{"project":{"command":"project-tool"}}}},
  "custom" : [1, 2],
  "mcpServers": {"local": { "command" : "user-tool" }}
}"#;

struct Fixture {
    root: tempfile::TempDir,
    accounts: CredentialFileStore,
    tenant: TenantScope,
    tx: ProviderTransactions,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let accounts = CredentialFileStore::new(root.path()).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        Self { root, accounts, tenant, tx }
    }

    fn account(&self, owner: &str) -> (CredentialLease, PathBuf) {
        let store = self.accounts.for_owner(owner).unwrap();
        store.register_claude_directory(&self.tenant, "host", "account").unwrap();
        let lease = store.lease(&self.tenant, "host", AppId::Claude, "account", false).unwrap();
        let AccountCredential::ClaudeDirectory(directory) = lease.read().unwrap() else {
            panic!("expected a scoped Claude directory");
        };
        let profile = directory.path().join(".claude.json");
        fs::write(&profile, BEFORE).unwrap();
        (lease, profile)
    }
}

fn profile_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn cc_mcp_t11_http_projects_claude_wire_type() {
    let fixture = Fixture::new();
    let (account, path) = fixture.account("member");
    let definition =
        parse_native(json!({"type":"http","url":"http://127.0.0.1:12345/mcp"})).unwrap();
    apply(&account, &fixture.tx, "managed", ClaudeMcpEdit::Insert(&definition), || Ok(())).unwrap();

    assert_eq!(
        profile_json(&path)["mcpServers"]["managed"]["type"],
        "http",
        "Claude requires the native HTTP type"
    );
}

#[test]
fn cc_mcp_t11_mirror_lifecycle_isolates_accounts_and_preserves_user_config() {
    let fixture = Fixture::new();
    let (account, profile) = fixture.account("member");
    let (other, other_profile) = fixture.account("other-member");
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());

    let program = fixture.root.path().join("stdio-fixture");
    let marker = fixture.root.path().join("ran");
    fs::write(&program, "#!/bin/sh\n: > \"$1\"\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    let stdio = parse_native(json!({
        "type":"stdio","command":program,"args":[marker, " spaced argument "],
        "env":{"FIXTURE_MODE":"plain"},"cwd":fixture.root.path()
    }))
    .unwrap();
    let edit = |edit| apply(&account, &fixture.tx, "managed", edit, || Ok(())).unwrap();

    edit(ClaudeMcpEdit::Insert(&stdio));
    assert!(read(&account).unwrap()["managed"].as_ref().is_ok_and(|value| value == &stdio));
    assert_eq!(fs::read_to_string(&other_profile).unwrap(), BEFORE);
    assert!(!read(&other).unwrap().contains_key("managed"));

    // The native client added a field unknown to the managed connection schema.
    let mut with_extension = profile_json(&profile);
    with_extension["mcpServers"]["managed"]["clientExtension"] = json!({"keep":true});
    fs::write(&profile, with_extension.to_string()).unwrap();
    let http = parse_native(json!({"type":"http","url":url,"headers":{"X-Test":"plain"}})).unwrap();
    edit(ClaudeMcpEdit::Update(&http));
    assert_eq!(
        profile_json(&profile)["mcpServers"]["managed"],
        json!({"type":"http","url":url,"headers":{"X-Test":"plain"},"clientExtension":{"keep":true}})
    );
    assert!(read(&account).unwrap()["managed"].as_ref().is_ok_and(|value| value == &http));

    edit(ClaudeMcpEdit::Remove);
    assert!(!read(&account).unwrap().contains_key("managed"), "disable removes the native entry");

    let sse = parse_native(json!({"type":"sse","url":url})).unwrap();
    edit(ClaudeMcpEdit::Insert(&sse));
    assert!(read(&account).unwrap()["managed"].as_ref().is_ok_and(|value| value == &sse));
    apply(&other, &fixture.tx, "managed", ClaudeMcpEdit::Insert(&sse), || Ok(())).unwrap();
    edit(ClaudeMcpEdit::Remove);
    edit(ClaudeMcpEdit::Remove);
    assert!(read(&other).unwrap()["managed"].as_ref().is_ok_and(|value| value == &sse));
    assert_eq!(profile_json(&profile), serde_json::from_str::<Value>(BEFORE).unwrap());
    assert_eq!(fs::metadata(profile).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(!marker.exists());
    assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[test]
fn cc_mcp_t11_invalid_entries_and_conflicts_are_structured() {
    let fixture = Fixture::new();
    let (account, profile) = fixture.account("member");
    let mut root = profile_json(&profile);
    root["mcpServers"]["future"] = json!({"type":"websocket","url":"ws://127.0.0.1"});
    root["mcpServers"]["broken"] = json!({"type":"stdio"});
    let before = root.to_string();
    fs::write(&profile, &before).unwrap();

    let imported = read(&account).unwrap();
    assert_eq!(imported["future"].as_ref().err(), Some(&ClaudeMcpError::UnsupportedTransport));
    assert_eq!(imported["broken"].as_ref().err(), Some(&ClaudeMcpError::InvalidDefinition));
    assert!(imported["local"].is_ok());

    let stdio = parse_native(json!({"command":"fixture"})).unwrap();
    assert_eq!(
        apply(&account, &fixture.tx, "local", ClaudeMcpEdit::Insert(&stdio), || Ok(())),
        Err(ClaudeMcpError::File(WriteError::Conflict))
    );
    assert_eq!(
        apply(&account, &fixture.tx, "managed", ClaudeMcpEdit::Insert(&stdio), || {
            Err(WriteError::Conflict)
        }),
        Err(ClaudeMcpError::File(WriteError::Conflict))
    );
    let unresolved: vw_api_types::mcp_management::McpConnectionDefinition =
        serde_json::from_value(json!({
            "transport":"stdio","command":"fixture","env":{"TOKEN":{
                "kind":"credential","reference":{"service":"claude_official","accountId":"fixture"}
            }}
        }))
        .unwrap();
    assert_eq!(
        apply(&account, &fixture.tx, "managed", ClaudeMcpEdit::Insert(&unresolved), || Ok(())),
        Err(ClaudeMcpError::CredentialUnavailable)
    );
    assert_eq!(fs::read_to_string(profile).unwrap(), before);
}

#[test]
fn cc_mcp_t11_ambient_credentials_and_wrong_host_cannot_publish() {
    let fixture = Fixture::new();
    let (account, profile) = fixture.account("member");
    let wrong_host =
        ProviderTransactions::new(fixture.root.path(), ExecutionHostId::new("other-host").unwrap())
            .unwrap();
    let stdio = parse_native(json!({"command":"fixture"})).unwrap();
    assert_eq!(
        apply(&account, &wrong_host, "managed", ClaudeMcpEdit::Insert(&stdio), || Ok(())),
        Err(ClaudeMcpError::File(WriteError::InvalidResource))
    );
    let store = fixture.accounts.for_owner("token-owner").unwrap();
    store
        .save(
            &fixture.tenant,
            "host",
            AppId::Claude,
            "token",
            &AccountCredential::ClaudeToken("fixture-token".into()),
        )
        .unwrap();
    let token = store.lease(&fixture.tenant, "host", AppId::Claude, "token", false).unwrap();
    assert!(matches!(read(&token), Err(WriteError::InvalidResource)));

    let ambient =
        fixture.accounts.lease(&fixture.tenant, "host", AppId::Claude, "account", true).unwrap();
    assert!(matches!(read(&ambient), Err(WriteError::InvalidResource)));
    assert_eq!(fs::read_to_string(profile).unwrap(), BEFORE);
}

#[test]
fn cc_mcp_t11_replaced_account_directory_rejects_publication() {
    let fixture = Fixture::new();
    let (account, profile) = fixture.account("member");
    let directory = profile.parent().unwrap();
    let retired = directory.with_extension("retired");
    let stdio = parse_native(json!({"command":"fixture"})).unwrap();
    assert_eq!(
        apply(&account, &fixture.tx, "managed", ClaudeMcpEdit::Insert(&stdio), || {
            fs::rename(directory, &retired).unwrap();
            fs::DirBuilder::new().mode(0o700).create(directory).unwrap();
            fs::write(&profile, BEFORE).unwrap();
            Ok(())
        }),
        Err(ClaudeMcpError::File(WriteError::Conflict))
    );
    assert_eq!(fs::read_to_string(profile).unwrap(), BEFORE);
    assert_eq!(fs::read_to_string(retired.join(".claude.json")).unwrap(), BEFORE);
    assert!(fixture.tx.pending(AppId::Claude).unwrap().is_none());
}
