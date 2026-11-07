use super::{claude::ClaudeFile, live::commit::*, mode_apply::*};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, sync::Mutex};
use vw_api_types::{agent_apps::AppId, app_providers::mode_apply::*, id::TenantScope};
struct Runtime {
    path: std::path::PathBuf,
    published: Mutex<Vec<Option<String>>>,
}
impl RouteRuntime for Runtime {
    fn prepare(&self, _: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
        ClientEndpoint::new("http://127.0.0.1:12345")
    }
    fn publish(&self, _: &RouteScope<'_>, id: Option<&str>) -> Result<(), ModeApplyError> {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&self.path).unwrap()).unwrap();
        assert_eq!(
            value["env"]["ANTHROPIC_BASE_URL"],
            if id.is_some() { "http://127.0.0.1:12345" } else { "https://direct.example" }
        );
        self.published.lock().unwrap().push(id.map(str::to_owned));
        Ok(())
    }
}
fn row() -> serde_json::Value {
    json!({"env":{"ANTHROPIC_BASE_URL":"https://direct.example","ANTHROPIC_API_KEY":"fake","ANTHROPIC_MODEL":"model"}})
}
#[test]
fn cc04_mode_apply_publishes_state_after_files() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    tx.commit(
        &tenant,
        &mut file.guard,
        &[],
        PendingTarget { pointer: Some("direct".into()), ..Default::default() },
    )
    .unwrap();
    let runtime =
        Runtime { path: home.path().join(".claude/settings.json"), published: Mutex::new(vec![]) };
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: Some(&runtime),
    };
    let result = mode
        .apply(&ModeApplyAction::Proxy { provider_id: "route".into() }, &row(), || true)
        .unwrap();
    assert!(result.attached, "proxy application must write and attach before publishing");
    assert_eq!(*runtime.published.lock().unwrap(), vec![Some("route".into())]);
    assert_eq!(tx.state(AppId::Claude).unwrap().pointer.unwrap().id, "direct");
}
#[test]
fn cc04_mode_apply_detach_preserves_external_edits() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let runtime =
        Runtime { path: home.path().join(".claude/settings.json"), published: Mutex::new(vec![]) };
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: Some(&runtime),
    };
    let mut route = row();
    route["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("1");
    route["env"]["CLAUDE_CODE_MAX_OUTPUT_TOKENS"] = json!("131072");
    mode.apply(&ModeApplyAction::Proxy { provider_id: "route".into() }, &route, || true).unwrap();
    let mut live = json!({"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:12345","CLAUDE_CODE_DISABLE_ARTIFACT":"external","CLAUDE_CODE_MAX_OUTPUT_TOKENS":"131072"},"external":true});
    fs::write(&runtime.path, serde_json::to_vec(&live).unwrap()).unwrap();
    mode.apply(&ModeApplyAction::Detach {}, &row(), || true).unwrap();
    live = serde_json::from_slice(&fs::read(&runtime.path).unwrap()).unwrap();
    assert!(
        live["env"].get("CLAUDE_CODE_MAX_OUTPUT_TOKENS").is_none(),
        "matching owned value must be removed"
    );
    assert_eq!(live["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "external");
    assert_eq!(live["external"], true);
    let state = tx.state(AppId::Claude).unwrap();
    assert!(matches!(state.mode.unwrap().mode, Mode::Proxy));
}

#[test]
fn cc04_mode_apply_publication_failure_retries_without_rewriting() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Failing {
        first: AtomicBool,
    }
    impl RouteRuntime for Failing {
        fn prepare(&self, _: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
            ClientEndpoint::new("http://127.0.0.1:12345")
        }
        fn publish(&self, _: &RouteScope<'_>, _: Option<&str>) -> Result<(), ModeApplyError> {
            if self.first.swap(false, Ordering::SeqCst) {
                Err(ModeApplyError::Unavailable)
            } else {
                Ok(())
            }
        }
    }
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let runtime = Failing { first: AtomicBool::new(true) };
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: Some(&runtime),
    };
    let action = ModeApplyAction::Proxy { provider_id: "route".into() };
    assert_eq!(
        mode.apply(&action, &row(), || true).unwrap().target_publication,
        TargetPublication::Pending
    );
    let path = home.path().join(".claude/settings.json");
    let bytes = fs::read(&path).unwrap();
    let meta = fs::metadata(&path).unwrap();
    // Same client contract, different route: only publish the target.
    let result = mode
        .apply(&ModeApplyAction::Proxy { provider_id: "other".into() }, &row(), || true)
        .unwrap();
    assert_eq!(result.target_publication, TargetPublication::Published);
    assert!(!result.configuration_changed);
    use std::os::unix::fs::MetadataExt;
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().ino(), meta.ino());
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), meta.modified().unwrap());
    assert_eq!(
        tx.state(AppId::Claude).unwrap().mode.unwrap().proxy_route.as_deref(),
        Some("other")
    );
}
#[test]
fn cc04_mode_apply_recovery_preserves_external_edits_and_retries_publication() {
    use super::live::commit::{HOOK, Phase};
    use super::live::write_guard::WriteError;
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let runtime =
        Runtime { path: home.path().join(".claude/settings.json"), published: Mutex::new(vec![]) };
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: Some(&runtime),
    };
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(
            |_, _, phase| if phase == Phase::Published { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    let action = ModeApplyAction::Proxy { provider_id: "route".into() };
    assert!(mode.apply(&action, &row(), || true).is_err());
    HOOK.with_borrow_mut(|h| *h = None);
    assert!(runtime.published.lock().unwrap().is_empty());
    assert!(tx.pending(AppId::Claude).unwrap().unwrap().published);
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&runtime.path).unwrap()).unwrap();
    value["external"] = json!("preserve");
    fs::write(&runtime.path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(
        mode.apply(&action, &row(), || true),
        Err(ModeApplyError::Write(WriteError::Recovered))
    ));
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
    assert!(runtime.published.lock().unwrap().is_empty());
    assert_eq!(
        mode.apply(&action, &row(), || true).unwrap().target_publication,
        TargetPublication::Published
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&runtime.path).unwrap()).unwrap()["external"],
        "preserve"
    );
}
#[test]
fn cc04_mode_apply_validation_and_unavailable_runtime_leave_files_and_state_unchanged() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let runtime =
        Runtime { path: home.path().join(".claude/settings.json"), published: Mutex::new(vec![]) };
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: None,
    };
    let action = ModeApplyAction::Proxy { provider_id: "route".into() };
    assert!(matches!(mode.apply(&action, &row(), || true), Err(ModeApplyError::Unavailable)));
    mode.runtime = Some(&runtime);
    assert!(mode.apply(&action, &row(), || false).is_err());
    assert!(tx.state(AppId::Claude).unwrap().mode.is_none());
    assert!(!runtime.path.exists());
    assert!(runtime.published.lock().unwrap().is_empty());
    for url in [
        "https://remote.example:443",
        "http://127.0.0.1:1234/?token=x",
        "http://fake@127.0.0.1:1234",
        "http://127.0.0.1",
    ] {
        assert!(ClientEndpoint::new(url).is_err());
    }
}

#[test]
fn cc04_mode_apply_pending_login_cannot_be_bypassed() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let runtime =
        Runtime { path: home.path().join(".claude/settings.json"), published: Mutex::new(vec![]) };
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: Some(&runtime),
    };
    let journal = home.path().join(".claude/.shiju-login.json");
    fs::write(&journal, b"unfinished-login-owner").unwrap();
    assert!(
        mode.apply(&ModeApplyAction::Proxy { provider_id: "route".into() }, &row(), || true)
            .is_err()
    );
    assert_eq!(fs::read(&journal).unwrap(), b"unfinished-login-owner");
    assert!(tx.state(AppId::Claude).unwrap().mode.is_none());
    assert!(!runtime.path.exists());
    assert!(runtime.published.lock().unwrap().is_empty());
}
