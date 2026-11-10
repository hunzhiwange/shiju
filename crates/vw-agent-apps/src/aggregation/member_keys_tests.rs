use super::member_keys::allocate_key;
use crate::providers::{
    live::{
        commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
        write_guard::lock_app,
    },
    mode_state::StackState,
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[test]
fn aggregation_t03_member_keys_preserves_contract() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let host = ExecutionHostId::new("fixture-host").unwrap();
    let tx = ProviderTransactions::new(root.path(), host.clone()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let mut stack = StackState::default();
    let mut commit = |stack: &StackState| {
        tx.commit(
            &tenant,
            &mut guard,
            &[],
            PendingTarget { stack: Some(stack.clone()), ..Default::default() },
        )
        .unwrap();
    };

    assert_eq!(allocate_key(&mut stack, "first", "Same Relay", None), "same-relay");
    assert_eq!(allocate_key(&mut stack, "second", "Same Relay", None), "same-relay-2");
    stack.members = vec!["first".into(), "second".into()];
    commit(&stack);

    stack.members.remove(0);
    commit(&stack);
    drop(guard);
    let reopened = ProviderTransactions::open_existing(root.path(), host).unwrap().unwrap();
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let mut stack = reopened.state(AppId::Claude).unwrap().stack;
    assert_eq!(stack.members, ["second"]);
    assert_eq!(stack.keys.get("same-relay").map(String::as_str), Some("first"));
    assert_eq!(allocate_key(&mut stack, "third", "Same Relay", None), "same-relay-3");
    assert_eq!(allocate_key(&mut stack, "first", "Renamed Relay", Some("new-icon")), "same-relay");
    stack.members.extend(["third".into(), "first".into()]);
    reopened
        .commit(
            &tenant,
            &mut guard,
            &[],
            PendingTarget { stack: Some(stack), ..Default::default() },
        )
        .unwrap();
    let saved = reopened.state(AppId::Claude).unwrap().stack;
    assert_eq!(saved.members, ["second", "third", "first"]);
    assert_eq!(saved.keys.len(), 3);
    assert_eq!(saved.keys.get("same-relay").map(String::as_str), Some("first"));
}

#[test]
fn aggregation_t03_member_keys_normalizes_source_names() {
    let mut stack = StackState::default();
    for (id, name, icon, expected) in [
        ("a", "Other Name", Some("Kimi__For/Coding"), "kimi-for-coding"),
        ("b", "My Relay 2.0!", None, "my-relay-2-0"),
        ("c1b2-c3d4", "智谱", None, "pc1b2c3"),
        ("d", "DeepSeek", Some("  "), "deepseek"),
        ("e", "Acme", Some("data:image/png;base64,iVBORw0KGgo="), "acme"),
        ("f", "--A//中__B--", Some("厂商"), "a-b"),
        ("g", "ABCDEFGHIJKLMNOPQRSTUVW-XYZ", None, "abcdefghijklmnopqrstuvw"),
        ("h", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", None, "abcdefghijklmnopqrstuvwx"),
        ("i", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", None, "abcdefghijklmnopqrstuvwx-2"),
    ] {
        assert_eq!(allocate_key(&mut stack, id, name, icon), expected, "{id}");
    }
}
