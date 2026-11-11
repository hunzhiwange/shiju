use super::{claude_publish::publish, preview::*};
use crate::providers::{
    claude::ClaudeFile, live::commit::*, mode_apply::*, mode_state::StackState,
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId, app_providers::mode_apply::TargetPublication, id::TenantScope,
};

struct Runtime;
impl RouteRuntime for Runtime {
    fn prepare(&self, _: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
        ClientEndpoint::new("http://127.0.0.1:12345")
    }
    fn publish(&self, _: &RouteScope<'_>, provider: Option<&str>) -> Result<(), ModeApplyError> {
        assert_eq!(provider, Some("alpha"));
        Ok(())
    }
}
fn input<'a>(providers: &'a [Provider<'a>]) -> PreviewInput<'a> {
    PreviewInput {
        candidate_default: Some("alpha"),
        providers,
        pending_tenant: None,
        failover_enabled: false,
        direct_restorable: true,
        catalog_policy: CatalogPolicy::Preserve,
    }
}
#[test]
fn aggregation_t14_claude_publish_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let path = home.path().join(".claude/settings.json");
    fs::write(&path, br#"{ "hooks": {"user":true}, "env":{"USER_FLAG":"keep","AWS_SECRET_ACCESS_KEY":"fake-old"}, "model":"old" }"#).unwrap();
    tx.commit(
        &tenant,
        &mut file.guard,
        &[],
        PendingTarget {
            pointer: Some("direct".into()),
            stack: Some(StackState {
                enabled: false,
                members: vec!["alpha".into()],
                keys: [("a".into(), "alpha".into())].into(),
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let record = json!({"name":"Alpha","settingsConfig":{"env":{"ANTHROPIC_MODEL":"model[1M]","CLAUDE_CODE_DISABLE_THINKING":"1"}}});
    let providers = [Provider {
        id: "alpha",
        name: "Alpha",
        official_service: None,
        catalog: Catalog::Claude(&record),
    }];
    let project = vw_api_types::id::ProjectScope::new(
        tenant.clone(),
        vw_api_types::id::ProjectId::from("project"),
    )
    .unwrap();
    let mut mode = ClaudeMode {
        file: &mut file,
        transactions: &tx,
        scope: RouteScope { tenant: &tenant, host: "host", app: AppId::Claude },
        runtime: Some(&Runtime),
    };
    let result = publish(
        &mut mode,
        &project,
        &tx.state(AppId::Claude).unwrap(),
        true,
        input(&providers),
        || true,
    )
    .unwrap();
    assert!(result.attached, "aggregation publication must attach the client");
    assert_eq!(result.target_publication, TargetPublication::Published);
    let bytes = fs::read(&path).unwrap();
    let live: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(live["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"], "ccs-claude-a--model[1M]");
    assert_eq!(live["env"]["ANTHROPIC_AUTH_TOKEN"], CLIENT_TOKEN_PLACEHOLDER);
    assert_eq!(live["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:12345");
    assert_eq!(live["env"]["USER_FLAG"], "keep");
    assert!(live["env"].get("AWS_SECRET_ACCESS_KEY").is_none());
    assert!(live.get("model").is_none());
    assert_eq!(live["modelPicker"]["options"][0]["model"], "ccs-claude-a--model[1M]");
    assert!(String::from_utf8(bytes.clone()).unwrap().contains("\"hooks\": {\"user\":true}"));
    let state = tx.state(AppId::Claude).unwrap();
    assert!(state.stack.enabled);
    assert_eq!(state.pointer.as_ref().unwrap().id, "direct");
    let result = publish(&mut mode, &project, &state, false, input(&providers), || true).unwrap();
    assert!(!result.configuration_changed);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(tx.state(AppId::Claude).unwrap() == state);
}

struct Fixture {
    home: tempfile::TempDir,
    _private: tempfile::TempDir,
    tx: ProviderTransactions,
    file: ClaudeFile,
    tenant: TenantScope,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
        let tenant = TenantScope::new("p", "c").unwrap();
        let mut file = ClaudeFile::open(home.path(), true).unwrap();
        fs::write(
            home.path().join(".claude/settings.json"),
            br#"{"hooks":{"keep":true},"env":{"USER_FLAG":"keep"}}"#,
        )
        .unwrap();
        tx.commit(
            &tenant,
            &mut file.guard,
            &[],
            PendingTarget {
                pointer: Some("direct".into()),
                stack: Some(StackState {
                    members: vec!["alpha".into(), "beta".into()],
                    keys: [("a".into(), "alpha".into()), ("b".into(), "beta".into())].into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        Self { home, _private: private, tx, file, tenant }
    }
    fn project(&self, id: &str) -> vw_api_types::id::ProjectScope {
        vw_api_types::id::ProjectScope::new(
            self.tenant.clone(),
            vw_api_types::id::ProjectId::from(id),
        )
        .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        fs::read(self.home.path().join(".claude/settings.json")).unwrap()
    }
}
fn records() -> [Value; 2] {
    [
        json!({"name":"Alpha","settingsConfig":{"env":{"ANTHROPIC_MODEL":"one[1M]","CLAUDE_CODE_DISABLE_THINKING":"1","ENABLE_TOOL_SEARCH":"1","CLAUDE_CODE_MAX_CONTEXT_TOKENS":"999999"}}}),
        json!({"name":"Beta","settingsConfig":{"env":{"ANTHROPIC_MODEL":"two","CLAUDE_CODE_MAX_CONTEXT_TOKENS":"128000","CLAUDE_CODE_DISABLE_ARTIFACT":"yes"}}}),
    ]
}
fn providers(records: &[Value; 2]) -> [Provider<'_>; 2] {
    [
        Provider {
            id: "alpha",
            name: "Alpha",
            official_service: None,
            catalog: Catalog::Claude(&records[0]),
        },
        Provider {
            id: "beta",
            name: "Beta",
            official_service: None,
            catalog: Catalog::Claude(&records[1]),
        },
    ]
}
#[test]
fn aggregation_t14_publish_merges_compatibility_and_catalog_contract() {
    let mut f = Fixture::new();
    let project = f.project("project");
    let records = records();
    let providers = providers(&records);
    let expected = f.tx.state(AppId::Claude).unwrap();
    let mut mode = ClaudeMode {
        file: &mut f.file,
        transactions: &f.tx,
        scope: RouteScope { tenant: &f.tenant, host: "host", app: AppId::Claude },
        runtime: Some(&Runtime),
    };
    publish(&mut mode, &project, &expected, true, input(&providers), || true).unwrap();
    let live: Value = serde_json::from_slice(&f.bytes()).unwrap();
    assert_eq!(live["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"], "128000");
    assert_eq!(live["env"]["CLAUDE_CODE_DISABLE_THINKING"], "1");
    assert_eq!(live["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "yes");
    assert!(live["env"].get("ENABLE_TOOL_SEARCH").is_none());
    assert_eq!(live["modelPicker"]["options"].as_array().unwrap().len(), 2);
    assert_eq!(live["modelPicker"]["options"][1]["model"], "ccs-claude-b--two");
    assert_eq!(live["env"]["USER_FLAG"], "keep");
    f.tx.state(AppId::Claude).unwrap().require_project(&project).unwrap();
}
#[test]
fn aggregation_t14_publish_rejects_unready_unauthorized_and_stale_inputs() {
    let mut f = Fixture::new();
    let a = f.project("a");
    let b = f.project("b");
    let records = records();
    let providers = providers(&records);
    let state = f.tx.state(AppId::Claude).unwrap();
    let before = f.bytes();
    let mut mode = ClaudeMode {
        file: &mut f.file,
        transactions: &f.tx,
        scope: RouteScope { tenant: &f.tenant, host: "host", app: AppId::Claude },
        runtime: None,
    };
    assert!(matches!(
        publish(&mut mode, &a, &state, true, input(&providers), || true),
        Err(ModeApplyError::Unavailable)
    ));
    mode.runtime = Some(&Runtime);
    assert!(matches!(
        publish(&mut mode, &a, &state, false, input(&providers), || true),
        Err(ModeApplyError::Ownership)
    ));
    let mut conflicting = input(&providers);
    conflicting.failover_enabled = true;
    assert!(matches!(
        publish(&mut mode, &a, &state, true, conflicting, || true),
        Err(ModeApplyError::InvalidInput)
    ));
    let mut unrestorable = input(&providers);
    unrestorable.direct_restorable = false;
    assert!(publish(&mut mode, &a, &state, true, unrestorable, || true).is_err());
    assert!(matches!(
        publish(&mut mode, &a, &state, true, input(&providers), || false),
        Err(ModeApplyError::Write(crate::providers::live::write_guard::WriteError::Conflict))
    ));
    assert_eq!(fs::read(f.home.path().join(".claude/settings.json")).unwrap(), before);
    publish(&mut mode, &a, &state, true, input(&providers), || true).unwrap();
    let bound = f.tx.state(AppId::Claude).unwrap();
    assert!(matches!(
        publish(&mut mode, &b, &bound, true, input(&providers), || true),
        Err(ModeApplyError::Ownership)
    ));
    assert!(publish(&mut mode, &a, &state, true, input(&providers), || true).is_err());
}
#[test]
fn aggregation_t14_publish_failure_keeps_original_or_recovers_frozen_intent() {
    use crate::providers::live::{
        commit::{HOOK, Phase},
        write_guard::WriteError,
    };
    for phase in [Phase::Intent, Phase::Marked, Phase::Published] {
        let mut f = Fixture::new();
        let project = f.project("project");
        let records = records();
        let providers = providers(&records);
        let state = f.tx.state(AppId::Claude).unwrap();
        let before = f.bytes();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let mut mode = ClaudeMode {
            file: &mut f.file,
            transactions: &f.tx,
            scope: RouteScope { tenant: &f.tenant, host: "host", app: AppId::Claude },
            runtime: Some(&Runtime),
        };
        let result = publish(&mut mode, &project, &state, true, input(&providers), || true);
        HOOK.with_borrow_mut(|h| *h = None);
        assert!(matches!(result, Err(ModeApplyError::Write(WriteError::Io))));
        assert!(f.tx.pending(AppId::Claude).unwrap().is_some());
        f.tx.recover_project(&project, &mut f.file.guard).unwrap();
        if phase == Phase::Intent {
            assert_eq!(f.bytes(), before);
            assert!(f.tx.state(AppId::Claude).unwrap() == state);
        } else {
            let live: Value = serde_json::from_slice(&f.bytes()).unwrap();
            assert_eq!(live["modelPicker"]["options"][0]["model"], "ccs-claude-a--one[1M]");
            f.tx.state(AppId::Claude).unwrap().require_project(&project).unwrap();
        }
    }
}
#[test]
fn aggregation_t14_route_failure_is_pending_and_retries_after_file_commit() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Retry<'a> {
        tx: &'a ProviderTransactions,
        path: std::path::PathBuf,
        first: AtomicBool,
    }
    impl RouteRuntime for Retry<'_> {
        fn prepare(&self, _: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
            ClientEndpoint::new("http://127.0.0.1:12345")
        }
        fn publish(&self, _: &RouteScope<'_>, _: Option<&str>) -> Result<(), ModeApplyError> {
            assert!(self.tx.state(AppId::Claude).unwrap().stack.enabled);
            let live: Value = serde_json::from_slice(&fs::read(&self.path).unwrap()).unwrap();
            assert!(live["modelPicker"].is_object());
            if self.first.swap(false, Ordering::SeqCst) {
                Err(ModeApplyError::Unavailable)
            } else {
                Ok(())
            }
        }
    }
    let mut f = Fixture::new();
    let project = f.project("project");
    let records = records();
    let providers = providers(&records);
    let state = f.tx.state(AppId::Claude).unwrap();
    let runtime = Retry {
        tx: &f.tx,
        path: f.home.path().join(".claude/settings.json"),
        first: AtomicBool::new(true),
    };
    let mut mode = ClaudeMode {
        file: &mut f.file,
        transactions: &f.tx,
        scope: RouteScope { tenant: &f.tenant, host: "host", app: AppId::Claude },
        runtime: Some(&runtime),
    };
    let result = publish(&mut mode, &project, &state, true, input(&providers), || true).unwrap();
    assert_eq!(result.target_publication, TargetPublication::Pending);
    let bytes = fs::read(&runtime.path).unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    let result = publish(&mut mode, &project, &state, false, input(&providers), || true).unwrap();
    assert_eq!(result.target_publication, TargetPublication::Published);
    assert!(!result.configuration_changed);
    assert_eq!(fs::read(&runtime.path).unwrap(), bytes);
}
#[test]
fn aggregation_t14_late_revision_conflict_discards_unpublished_intent() {
    use crate::providers::live::write_guard::WriteError;
    let mut f = Fixture::new();
    let project = f.project("project");
    let records = records();
    let providers = providers(&records);
    let state = f.tx.state(AppId::Claude).unwrap();
    let before = f.bytes();
    let checks = std::cell::Cell::new(0);
    let mut mode = ClaudeMode {
        file: &mut f.file,
        transactions: &f.tx,
        scope: RouteScope { tenant: &f.tenant, host: "host", app: AppId::Claude },
        runtime: Some(&Runtime),
    };
    let result = publish(&mut mode, &project, &state, true, input(&providers), || {
        checks.set(checks.get() + 1);
        checks.get() < 3
    });
    assert!(matches!(result, Err(ModeApplyError::Write(WriteError::Conflict))));
    assert_eq!(checks.get(), 3);
    assert!(!f.tx.pending(AppId::Claude).unwrap().unwrap().published);
    f.tx.recover_project(&project, &mut f.file.guard).unwrap();
    assert_eq!(f.bytes(), before);
    assert!(f.tx.state(AppId::Claude).unwrap() == state);
}
