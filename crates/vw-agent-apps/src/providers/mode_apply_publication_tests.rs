use super::{
    claude::ClaudeFile,
    live::{commit::*, write_guard::WriteError},
    mode_apply::{ClaudeMode, RouteScope},
};
use serde_json::{Value, json};
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::mode_apply::ModeApplyAction,
    id::{ProjectScope, TenantScope},
};

struct ResetHook;
impl Drop for ResetHook {
    fn drop(&mut self) {
        HOOK.with_borrow_mut(|hook| *hook = None);
    }
}

#[test]
fn ha_enable_publication_recovers_owned_plain_intent() {
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tenant = TenantScope::new("p", "owner").unwrap();
        let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
        let foreign = ProjectScope::new(tenant.clone(), "other".into()).unwrap();
        let tx = ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
        let mut file = ClaudeFile::open(home.path(), true).unwrap();
        tx.commit(
            &tenant,
            &mut file.guard,
            &[],
            PendingTarget { pointer: Some("direct".into()), ..Default::default() },
        )
        .unwrap();
        let expected = tx.state(AppId::Claude).unwrap();
        let scope = RouteScope { tenant: &tenant, host: "host", app: AppId::Claude };
        let mut mode = ClaudeMode { file: &mut file, transactions: &tx, scope, runtime: None };
        let target = json!({"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:12345","ANTHROPIC_AUTH_TOKEN":"fixture-local-token"}});
        let action = ModeApplyAction::Proxy { provider_id: "route".into() };
        let _reset = ResetHook;
        HOOK.with_borrow_mut(|hook| {
            *hook = Some(Box::new(
                move |_, _, current| {
                    if current == phase { Err(WriteError::Io) } else { Ok(()) }
                },
            ));
        });
        assert!(mode.commit_route(&project, &expected, &action, &target, || true).is_err());
        HOOK.with_borrow_mut(|hook| *hook = None);
        if tx.pending(AppId::Claude).unwrap().is_some() {
            let before = tx.state(AppId::Claude).unwrap();
            let bytes = mode.file.guard.read(OsStr::new(mode.file.name)).unwrap();
            assert!(mode.recover_route(&foreign).is_err());
            assert!(tx.state(AppId::Claude).unwrap() == before);
            assert_eq!(mode.file.guard.read(OsStr::new(mode.file.name)).unwrap(), bytes);
            assert!(tx.pending(AppId::Claude).unwrap().is_some());
            mode.recover_route(&project).unwrap();
        }
        assert!(tx.pending(AppId::Claude).unwrap().is_none());
        // Recovery may roll back an unpublished intent or finish its frozen files.
        // A fresh request can always commit using the new durable preimage.
        let state = tx.state(AppId::Claude).unwrap();
        mode.commit_route(&project, &state, &action, &target, || true).unwrap();
        let state = tx.state(AppId::Claude).unwrap();
        state.require_project(&project).unwrap();
        assert_eq!(state.pointer.as_ref().unwrap().id, "direct");
        assert!(state.mode.as_ref().unwrap().attached);
        assert_eq!(state.mode.as_ref().unwrap().proxy_route.as_deref(), Some("route"));
        let bytes = mode.file.guard.read(OsStr::new(mode.file.name)).unwrap().unwrap();
        let actual: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(actual["env"]["ANTHROPIC_AUTH_TOKEN"], "fixture-local-token");
        assert_eq!(actual["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:12345");
    }
}
