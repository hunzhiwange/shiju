use super::native_file::{JsonServerEdit, JsonServerPatch, NativeFile};
use crate::providers::live::{
    commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions},
    patch_json::object_value,
    recovery::RecoveryOutcome,
    stage::{FileChange, FilePatch},
    write_guard::{FilePrivacy, WriteError, lock_app},
};
use serde_json::json;
use std::{
    cell::Cell,
    ffi::OsStr,
    fs::{self, File},
    io::Read,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

const FILE: &str = "settings.json";
const BEFORE: &[u8] = br#"{"mcpServers":{"managed":{"command":"old"},"user":{"command":"keep"}}}"#;
const EXTERNAL: &[u8] =
    br#"{"mcpServers":{"managed":{"command":"old"},"user":{"command":"external"}}}"#;

struct Fixture {
    root: tempfile::TempDir,
    live: tempfile::TempDir,
    transactions: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let transactions =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::write(live.path().join(FILE), bytes).unwrap();
        Self { root, live, transactions, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn path(&self) -> PathBuf {
        self.live.path().join(FILE)
    }
}

#[test]
fn cc_mcp_t10_stale_file_revision_rejects_publication() {
    let fixture = Fixture::new(BEFORE);
    let mut guard = lock_app(AppId::Mcode, fixture.live.path()).unwrap();
    let file = NativeFile::new(&mut guard, OsStr::new(FILE));
    let expected = file.read().unwrap();
    fs::write(fixture.path(), EXTERNAL).unwrap();
    let fields = json!({"command": "new"});
    let patch = update_command(&fields);

    assert_eq!(
        file.commit(&fixture.transactions, &fixture.tenant, expected.as_deref(), &patch, || Ok(())),
        Err(WriteError::Conflict),
        "a changed native snapshot must reject publication"
    );
    assert_eq!(fs::read(fixture.path()).unwrap(), EXTERNAL);
    assert!(fixture.transactions.pending(AppId::Mcode).unwrap().is_none());
}

fn update_command(fields: &serde_json::Value) -> JsonServerPatch<'_> {
    JsonServerPatch {
        section: "mcpServers",
        id: "managed",
        edit: JsonServerEdit::Update { set: fields.as_object().unwrap(), remove: &["args"] },
    }
}

fn publish(fixture: &Fixture, patch: &dyn FilePatch) -> Result<(), WriteError> {
    let mut guard = lock_app(AppId::Mcode, fixture.live.path()).unwrap();
    let file = NativeFile::new(&mut guard, OsStr::new(FILE));
    let expected = file.read()?;
    file.commit(&fixture.transactions, &fixture.tenant, expected.as_deref(), patch, || Ok(()))
}

#[test]
fn cc_mcp_t10_atomic_replace_preserves_user_fields_and_services() {
    let before = br#"{
        // keep this comment
        "mcpServers": {
            "managed": {"command":"old", "args":["old"], "x-user" : true},
            "user": { "command" : "keep" },
        },
        "custom" : [1, 2]
    }"#;
    let fixture = Fixture::new(before);
    let mut old_handle = File::open(fixture.path()).unwrap();
    let fields = json!({"command": "new"});
    publish(&fixture, &update_command(&fields)).unwrap();

    let mut original = Vec::new();
    old_handle.read_to_end(&mut original).unwrap();
    assert_eq!(original, before, "open readers must keep the original file after rename");
    let after = fs::read(fixture.path()).unwrap();
    let parsed = object_value(&after).unwrap();
    assert_eq!(parsed["mcpServers"]["managed"], json!({"command":"new","x-user":true}));
    let after = String::from_utf8(after).unwrap();
    for preserved in [
        "// keep this comment",
        r#""x-user" : true"#,
        r#""user": { "command" : "keep" }"#,
        r#""custom" : [1, 2]"#,
    ] {
        assert!(after.contains(preserved));
    }
    assert_eq!(fs::metadata(fixture.path()).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(fixture.transactions.pending(AppId::Mcode).unwrap().is_none());
}

#[test]
fn cc_mcp_t10_bad_config_preserves_original_bytes() {
    let fields = json!({"command": "new"});
    for bytes in
        [b"{broken".as_slice(), br#"{"mcpServers":[]}"#, br#"{"mcpServers":{"managed":null}}"#]
    {
        let fixture = Fixture::new(bytes);
        assert_eq!(publish(&fixture, &update_command(&fields)), Err(WriteError::InvalidResource));
        assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
        assert!(fixture.transactions.pending(AppId::Mcode).unwrap().is_none());
    }
}

#[test]
fn cc_mcp_t10_same_name_unmanaged_server_rejects_insert() {
    let fixture = Fixture::new(BEFORE);
    let fields = json!({"command": "new"});
    let patch = JsonServerPatch {
        section: "mcpServers",
        id: "user",
        edit: JsonServerEdit::Insert(fields.as_object().unwrap()),
    };
    assert_eq!(publish(&fixture, &patch), Err(WriteError::Conflict));
    assert_eq!(fs::read(fixture.path()).unwrap(), BEFORE);
}

#[test]
fn cc_mcp_t10_insert_and_revoke_only_touch_the_selected_service() {
    let fixture = Fixture::new(BEFORE);
    let fields = json!({"command": "new"});
    let mut patch = JsonServerPatch {
        section: "mcpServers",
        id: "added",
        edit: JsonServerEdit::Insert(fields.as_object().unwrap()),
    };
    publish(&fixture, &patch).unwrap();
    let inserted = object_value(&fs::read(fixture.path()).unwrap()).unwrap();
    assert_eq!(inserted["mcpServers"]["added"], fields);
    patch.edit = JsonServerEdit::Remove;
    publish(&fixture, &patch).unwrap();
    publish(&fixture, &patch).unwrap();
    assert_eq!(fs::read(fixture.path()).unwrap(), BEFORE);
}

#[test]
fn cc_mcp_t10_symlink_and_parent_escape_are_rejected() {
    let fixture = Fixture::new(BEFORE);
    let outside = fixture.root.path().join(FILE);
    fs::write(&outside, BEFORE).unwrap();
    fs::remove_file(fixture.path()).unwrap();
    symlink(&outside, fixture.path()).unwrap();
    let fields = json!({"command": "new"});
    assert_eq!(publish(&fixture, &update_command(&fields)), Err(WriteError::InvalidResource));
    let mut guard = lock_app(AppId::Mcode, fixture.live.path()).unwrap();
    assert_eq!(
        NativeFile::new(&mut guard, OsStr::new("../settings.json")).read(),
        Err(WriteError::InvalidResource)
    );
    assert_eq!(fs::read_link(fixture.path()).unwrap(), outside);
    assert_eq!(fs::read(outside).unwrap(), BEFORE);
}

#[test]
fn cc_mcp_t10_waits_for_provider_transaction_on_same_file() {
    let fixture = Fixture::new(BEFORE);
    let mut provider_guard = lock_app(AppId::Mcode, fixture.live.path()).unwrap();
    let (starting, started) = mpsc::channel();
    let (observing, observed) = mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            starting.send(()).unwrap();
            let mut guard = lock_app(AppId::Mcode, fixture.live.path()).unwrap();
            let file = NativeFile::new(&mut guard, OsStr::new(FILE));
            let expected = file.read().unwrap();
            observing.send(expected.clone()).unwrap();
            let fields = json!({"command": "new"});
            file.commit(
                &fixture.transactions,
                &fixture.tenant,
                expected.as_deref(),
                &update_command(&fields),
                || Ok(()),
            )
            .unwrap();
        });
        started.recv().unwrap();
        let waited = observed.recv_timeout(Duration::from_millis(100)).is_err();
        let provider_patch = |_: Option<&[u8]>| Ok(EXTERNAL.to_vec());
        let result = fixture.transactions.commit(
            &fixture.tenant,
            &mut provider_guard,
            &[FileChange {
                name: OsStr::new(FILE),
                privacy: FilePrivacy::Private,
                patch: &provider_patch,
            }],
            Default::default(),
        );
        drop(provider_guard);
        result.unwrap();
        assert!(waited);
        assert_eq!(observed.recv().unwrap().as_deref(), Some(EXTERNAL));
    });
    let parsed = object_value(&fs::read(fixture.path()).unwrap()).unwrap();
    assert_eq!(parsed["mcpServers"]["managed"]["command"], "new");
    assert_eq!(parsed["mcpServers"]["user"]["command"], "external");
}

#[test]
fn cc_mcp_t10_external_edit_during_publication_is_preserved() {
    let fixture = Fixture::new(BEFORE);
    let path = fixture.path();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish {
                fs::write(&path, EXTERNAL).unwrap();
            }
            Ok(())
        }));
    });
    let fields = json!({"command": "new"});
    let result = publish(&fixture, &update_command(&fields));
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(WriteError::Conflict));
    assert_eq!(fs::read(fixture.path()).unwrap(), EXTERNAL);
    assert!(fixture.transactions.pending(AppId::Mcode).unwrap().is_none());
}

#[test]
fn cc_mcp_t10_stale_saved_revision_discards_unpublished_intent() {
    let fixture = Fixture::new(BEFORE);
    let fields = json!({"command": "new"});
    let mut guard = lock_app(AppId::Mcode, fixture.live.path()).unwrap();
    let file = NativeFile::new(&mut guard, OsStr::new(FILE));
    let calls = Cell::new(0);
    assert_eq!(
        file.commit(
            &fixture.transactions,
            &fixture.tenant,
            Some(BEFORE),
            &update_command(&fields),
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 1 { Ok(()) } else { Err(WriteError::Conflict) }
            }
        ),
        Err(WriteError::Conflict)
    );
    assert_eq!(calls.get(), 2);
    assert_eq!(fs::read(fixture.path()).unwrap(), BEFORE);
    assert_eq!(fixture.transactions.recover(&mut guard).unwrap(), Some(RecoveryOutcome::Discarded));
    assert_eq!(fs::read(fixture.path()).unwrap(), BEFORE);
    assert!(fixture.transactions.pending(AppId::Mcode).unwrap().is_none());
}

#[test]
fn cc_mcp_t10_store_commit_failure_recovers_published_file() {
    let fixture = Fixture::new(BEFORE);
    let state = fixture.root.path().join("provider-live/aG9zdA/mcode.state.json");
    let obstructed = state.clone();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(move |_, _, phase| {
            if phase == Phase::Published {
                // Real filesystem failure at the transaction's state store.
                fs::create_dir(&obstructed).unwrap();
            }
            Ok(())
        }));
    });
    let fields = json!({"command": "new"});
    let result = publish(&fixture, &update_command(&fields));
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(WriteError::InvalidResource));
    assert!(fixture.transactions.pending(AppId::Mcode).unwrap().unwrap().published);
    let published = fs::read(fixture.path()).unwrap();
    let parsed = object_value(&published).unwrap();
    assert_eq!(parsed["mcpServers"]["managed"]["command"], "new");
    assert_eq!(parsed["mcpServers"]["user"]["command"], "keep");

    fs::remove_dir(&state).unwrap();
    let reopened =
        ProviderTransactions::new(fixture.root.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
    assert_eq!(
        reopened.recover_on_startup(),
        vec![(AppId::Mcode, Ok(RecoveryOutcome::RolledForward))]
    );
    assert!(state.is_file());
    assert!(reopened.pending(AppId::Mcode).unwrap().is_none());
    assert_eq!(fs::read(fixture.path()).unwrap(), published);
}
