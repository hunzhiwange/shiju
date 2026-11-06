use super::access::recover_provider_policy;
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt};
use vw_agent_apps::providers::live::{commit::*, stage::FileChange, write_guard::*};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[test]
fn cc04_recovery_executor_startup_consumes_trusted_host_policy() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let private = root.path().join("private");
    fs::create_dir(&private).unwrap();
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
    let live = root.path().join("live");
    fs::create_dir(&live).unwrap();
    let s = ProviderTransactions::new(&private, ExecutionHostId::new("h").unwrap()).unwrap();
    // A real filesystem failure after publication retains a durable pending.
    let state_path = private.join("provider-live/aA/claude.state.json");
    let mut guard = lock_app(AppId::Claude, &live).unwrap();
    let patch = |_: Option<&[u8]>| {
        // State was already read; publication can succeed before settlement fails.
        fs::create_dir(&state_path).unwrap();
        Ok(b"fixture-new".to_vec())
    };
    assert!(
        s.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &patch
            }],
            PendingTarget { pointer: Some("next".into()), mode: None }
        )
        .is_err()
    );
    assert!(s.pending(AppId::Claude).unwrap().unwrap().published);
    drop(guard);
    drop(s);
    fs::remove_dir(&state_path).unwrap();
    fs::write(live.join("config"), b"external").unwrap();
    let policy = root.path().join("access.json");
    fs::write(&policy, serde_json::to_vec(&serde_json::json!({
        "principals": [], "hosts": [{"id":"h", "online":true,"binding":"local",
        "home":root.path(),"applications":root.path(),"search_paths":[],"credential_root":private}]
    })).unwrap()).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    recover_provider_policy(&policy).unwrap();
    let restarted =
        ProviderTransactions::new(&private, ExecutionHostId::new("h").unwrap()).unwrap();
    assert!(restarted.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(restarted.state(AppId::Claude).unwrap().pointer.unwrap().id, "next");
    assert_eq!(fs::read(live.join("config")).unwrap(), b"external");
    assert_eq!(fs::metadata(state_path).unwrap().permissions().mode() & 0o777, 0o600);
    // Insecure administrator policy is rejected before opening transaction state.
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(recover_provider_policy(&policy).is_err());
    assert_eq!(fs::read(live.join("config")).unwrap(), b"external");
}
