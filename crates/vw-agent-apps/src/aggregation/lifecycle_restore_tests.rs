use super::{
    claude_publish,
    exit_restore::{Restore, RestoreError},
    lifecycle_restore::*,
    preview::{Catalog, CatalogPolicy, PreviewInput, Provider},
};
use crate::providers::{
    claude::ClaudeFile, live::commit::*, mode_apply::*, mode_state::StackState,
};
use serde_json::{Value, json};
use std::{cell::Cell, fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
};

struct Runtime(bool);
impl RouteRuntime for Runtime {
    fn prepare(&self, _: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
        if self.0 {
            return Err(ModeApplyError::Unavailable);
        }
        ClientEndpoint::new("http://127.0.0.1:23456")
    }
    fn publish(&self, _: &RouteScope<'_>, _: Option<&str>) -> Result<(), ModeApplyError> {
        Ok(())
    }
}
struct Fixture {
    home: tempfile::TempDir,
    private: tempfile::TempDir,
    tx: ProviderTransactions,
    file: ClaudeFile,
    project: ProjectScope,
    published: Value,
    direct: Value,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
        let project =
            ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project"))
                .unwrap();
        let mut file = ClaudeFile::open(home.path(), true).unwrap();
        let direct = json!({"env":{"ANTHROPIC_BASE_URL":"https://direct.example","ANTHROPIC_API_KEY":"fixture-direct"}});
        let mut live = direct.clone();
        live["hooks"] = json!({"user": true});
        fs::write(home.path().join(".claude/settings.json"), serde_json::to_vec(&live).unwrap())
            .unwrap();
        tx.commit_project_checked(
            &project,
            &mut file.guard,
            &AppState::default(),
            true,
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
            || Ok(()),
        )
        .unwrap();
        let mut f = Self { home, private, tx, file, project, published: Value::Null, direct };
        f.attach(false).unwrap();
        f.published = serde_json::from_slice(&f.bytes()).unwrap();
        f
    }
    fn bytes(&self) -> Vec<u8> {
        fs::read(self.home.path().join(".claude/settings.json")).unwrap()
    }
    fn state(&self) -> AppState {
        self.tx.state(AppId::Claude).unwrap()
    }
    fn detach(&mut self, cancelled: impl Fn() -> bool) -> Result<(), LifecycleError> {
        let state = self.state();
        detach_claude(
            &mut self.file,
            &self.tx,
            Restore {
                project: &self.project,
                expected: &state,
                published: &self.published,
                direct: &self.direct,
            },
            || true,
            cancelled,
        )
    }
    fn attach(
        &mut self,
        fail: bool,
    ) -> Result<vw_api_types::app_providers::mode_apply::ModeApplyResult, ModeApplyError> {
        let record = json!({"name":"Alpha","settingsConfig":{"env":{"ANTHROPIC_MODEL":"model"}}});
        let providers = [Provider {
            id: "alpha",
            name: "Alpha",
            official_service: None,
            catalog: Catalog::Claude(&record),
        }];
        let state = self.state();
        claude_publish::publish(
            &mut ClaudeMode {
                file: &mut self.file,
                transactions: &self.tx,
                scope: RouteScope {
                    tenant: self.project.tenant_scope(),
                    host: "host",
                    app: AppId::Claude,
                },
                runtime: Some(&Runtime(fail)),
            },
            &self.project,
            &state,
            false,
            PreviewInput {
                candidate_default: Some("alpha"),
                providers: &providers,
                pending_tenant: None,
                failover_enabled: false,
                direct_restorable: true,
                catalog_policy: CatalogPolicy::Preserve,
            },
            || true,
        )
    }
}
#[test]
fn aggregation_t18_lifecycle_restore_preserves_contract() {
    let mut f = Fixture::new();
    let before = f.state();
    f.detach(|| false).unwrap();
    let live: Value = serde_json::from_slice(&f.bytes()).unwrap();
    assert_eq!(live["env"], f.direct["env"]);
    assert!(live.get("modelPicker").is_none());
    assert_eq!(live["hooks"]["user"], true);
    let next = f.state();
    assert!(next.stack.enabled);
    assert_eq!(next.stack.members, before.stack.members);
    assert_eq!(next.stack.keys, before.stack.keys);
    assert!(next.pointer == before.pointer);
    assert!(matches!(next.mode.as_ref().unwrap().mode, Mode::Proxy));
    assert!(!next.mode.as_ref().unwrap().attached);
    assert_eq!(startup_action(&f.tx, &f.project, AppId::Claude), Ok(StartupAction::Attach));
    // Reopen the REAL persistent store as a fresh process would. Reattach uses
    // the original publisher, not a second lifecycle projection implementation.
    f.tx = ProviderTransactions::open_existing(
        f.private.path(),
        ExecutionHostId::new("host").unwrap(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(startup_action(&f.tx, &f.project, AppId::Claude), Ok(StartupAction::Attach));
    f.attach(false).unwrap();
    let attached: Value = serde_json::from_slice(&f.bytes()).unwrap();
    assert_eq!(attached["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:23456");
    assert!(!attached["modelPicker"]["options"].as_array().unwrap().is_empty());
    assert!(f.state().mode.unwrap().attached);
    let bytes = f.bytes();
    assert!(!f.attach(false).unwrap().configuration_changed);
    assert_eq!(f.bytes(), bytes);
}
#[test]
fn aggregation_t18_cancel_before_or_during_prepare_does_not_publish() {
    for late in [false, true] {
        let mut f = Fixture::new();
        let before = f.bytes();
        let state = f.state();
        let calls = Cell::new(0);
        assert_eq!(
            f.detach(|| {
                calls.set(calls.get() + 1);
                !late || calls.get() >= 3
            }),
            Err(LifecycleError::Cancelled)
        );
        assert_eq!(f.bytes(), before);
        assert!(f.state() == state);
        // Retrying an already cancelled request performs no file/state write.
        assert_eq!(f.detach(|| true), Err(LifecycleError::Cancelled));
        assert_eq!(f.bytes(), before);
        if f.tx.pending(AppId::Claude).unwrap().is_some() {
            f.tx.recover_project(&f.project, &mut f.file.guard).unwrap();
        }
    }
}
#[test]
fn aggregation_t18_partial_detach_keeps_intent_until_scoped_recovery() {
    use crate::providers::live::{recovery::RecoveryOutcome, write_guard::WriteError};
    for phase in [Phase::Marked, Phase::Published] {
        let mut f = Fixture::new();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let result = f.detach(|| false);
        HOOK.with_borrow_mut(|h| *h = None);
        assert_eq!(result, Err(LifecycleError::Restore(RestoreError::TargetFailed)));
        assert!(f.tx.pending(AppId::Claude).unwrap().unwrap().published);
        assert_eq!(
            startup_action(&f.tx, &f.project, AppId::Claude),
            Err(RestoreError::RecoveryPending)
        );
        assert_eq!(f.detach(|| false), Err(LifecycleError::Restore(RestoreError::RecoveryPending)));
        assert_eq!(
            f.tx.recover_project(&f.project, &mut f.file.guard).unwrap(),
            Some(RecoveryOutcome::RolledForward)
        );
        assert!(f.state().stack.enabled);
        assert!(!f.state().mode.unwrap().attached);
        assert_eq!(startup_action(&f.tx, &f.project, AppId::Claude), Ok(StartupAction::Attach));
    }
}
#[test]
fn aggregation_t18_failed_reattach_preserves_direct_files_and_saved_intent() {
    let mut f = Fixture::new();
    f.detach(|| false).unwrap();
    let before = f.bytes();
    let state = f.state();
    assert!(matches!(f.attach(true), Err(ModeApplyError::Unavailable)));
    assert_eq!(f.bytes(), before);
    assert!(f.state() == state);
    assert_eq!(startup_action(&f.tx, &f.project, AppId::Claude), Ok(StartupAction::Attach));
    f.attach(false).unwrap();
    assert!(f.state().mode.unwrap().attached);
}
#[test]
fn aggregation_t18_scope_user_edits_and_unsupported_apps_are_rejected_without_writes() {
    let mut f = Fixture::new();
    let before = f.bytes();
    let state = f.state();
    let foreign =
        ProjectScope::new(TenantScope::new("p", "other").unwrap(), ProjectId::from("project"))
            .unwrap();
    assert_eq!(startup_action(&f.tx, &foreign, AppId::Claude), Err(RestoreError::Forbidden));
    assert_eq!(
        startup_action(&f.tx, &f.project, AppId::Gemini),
        Err(RestoreError::UnsupportedTarget)
    );
    assert_eq!(
        detach_claude(
            &mut f.file,
            &f.tx,
            Restore {
                project: &foreign,
                expected: &state,
                published: &f.published,
                direct: &f.direct
            },
            || true,
            || false
        ),
        Err(LifecycleError::Restore(RestoreError::Forbidden))
    );
    assert_eq!(f.bytes(), before);
    let mut edited: Value = serde_json::from_slice(&before).unwrap();
    edited["modelPicker"] = json!({"options":[{"model":"user-new"}]});
    let bytes = serde_json::to_vec(&edited).unwrap();
    fs::write(f.home.path().join(".claude/settings.json"), &bytes).unwrap();
    assert_eq!(f.detach(|| false), Err(LifecycleError::Restore(RestoreError::RevisionConflict)));
    assert_eq!(f.bytes(), bytes);
    assert!(f.state() == state);
}
#[test]
fn aggregation_t18_explicit_exit_stays_direct_after_restart() {
    let mut f = Fixture::new();
    let state = f.state();
    super::exit_restore::restore_claude(
        &mut f.file,
        &f.tx,
        Restore {
            project: &f.project,
            expected: &state,
            published: &f.published,
            direct: &f.direct,
        },
        || true,
    )
    .unwrap();
    assert_eq!(startup_action(&f.tx, &f.project, AppId::Claude), Ok(StartupAction::StayDirect));
}
#[test]
fn aggregation_t18_codex_detach_restores_files_and_keeps_restart_intent() {
    use crate::providers::{
        codex_catalog::CodexFiles,
        codex_login::{AuthTarget, LoginInput},
    };
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let project =
        ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project")).unwrap();
    let mut files = CodexFiles::open(home.path()).unwrap();
    let config = "model='ccs-a/model'\nmodel_provider='custom'\nmodel_catalog_json='shiju-model-catalog.json'\n[model_providers.custom]\nbase_url='http://127.0.0.1:12345'\nwire_api='responses'\nexperimental_bearer_token='fixture-proxy'\n[agents]\ndefault_subagent_model='ccs-a/model'\n";
    let published = json!({"config":config,"catalog":{"models":[{"slug":"ccs-a/model"}]}});
    let direct = json!({"config":"model='main'\ncli_auth_credentials_store='file'\n", "auth":{}});
    fs::write(
        home.path().join(".codex/config.toml"),
        format!("# user comment\n{config}\n[history]\npersistence='save-all'\n"),
    )
    .unwrap();
    fs::write(
        home.path().join(".codex/shiju-model-catalog.json"),
        serde_json::to_vec(&published["catalog"]).unwrap(),
    )
    .unwrap();
    tx.commit_project_checked(
        &project,
        &mut files.guard,
        &AppState::default(),
        true,
        &[],
        PendingTarget {
            pointer: Some("direct".into()),
            stack: Some(StackState {
                enabled: true,
                members: vec!["alpha".into()],
                keys: [("a".into(), "alpha".into())].into(),
            }),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    let state = tx.state(AppId::Codex).unwrap();
    detach_codex(
        &mut files,
        &tx,
        Restore { project: &project, expected: &state, published: &published, direct: &direct },
        LoginInput {
            target: AuthTarget::Official { row_auth: &direct["auth"] },
            leaving_official: None,
            official_logins: &[],
            third_party_keys: &[],
            live_is_managed: false,
            now: 1_800_000_000,
        },
        None,
        || true,
        || false,
    )
    .unwrap();
    let after = tx.state(AppId::Codex).unwrap();
    assert!(after.stack == state.stack);
    assert!(after.pointer == state.pointer);
    assert!(!after.mode.as_ref().unwrap().attached);
    assert!(matches!(after.mode.as_ref().unwrap().mode, Mode::Proxy));
    let text = fs::read_to_string(home.path().join(".codex/config.toml")).unwrap();
    let live: Value = toml::from_str(&text).unwrap();
    assert_eq!(live["model"], "main");
    assert_eq!(live["history"]["persistence"], "save-all");
    assert!(text.contains("# user comment"));
    assert!(live.get("model_catalog_json").is_none());
    assert!(!home.path().join(".codex/shiju-model-catalog.json").exists());
    assert_eq!(startup_action(&tx, &project, AppId::Codex), Ok(StartupAction::Attach));
}

#[test]
fn aggregation_t18_desktop_startup_requests_republication_without_detaching_files() {
    let f = Fixture::new();
    let directory = tempfile::tempdir().unwrap();
    let mut guard =
        crate::providers::live::write_guard::lock_app(AppId::ClaudeDesktop, directory.path())
            .unwrap();
    let state = f.state();
    f.tx.commit_project_checked(
        &f.project,
        &mut guard,
        &AppState::default(),
        true,
        &[],
        PendingTarget {
            pointer: Some("direct".into()),
            stack: Some(state.stack),
            mode: state.mode,
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    fs::write(directory.path().join("client-owned"), b"unchanged").unwrap();
    assert_eq!(startup_action(&f.tx, &f.project, AppId::ClaudeDesktop), Ok(StartupAction::Attach));
    assert_eq!(fs::read(directory.path().join("client-owned")).unwrap(), b"unchanged");
    assert!(f.tx.state(AppId::ClaudeDesktop).unwrap().mode.unwrap().attached);
}
