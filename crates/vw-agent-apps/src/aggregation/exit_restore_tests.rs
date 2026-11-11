use super::exit_restore::*;
use crate::providers::{claude::ClaudeFile, live::commit::*, mode_state::StackState};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
};

#[test]
fn aggregation_t17_exit_restore_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tx =
        ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let project = ProjectScope::new(tenant, ProjectId::from("project")).unwrap();
    let mut file = ClaudeFile::open(home.path(), true).unwrap();
    let published = json!({"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:12345", "ANTHROPIC_AUTH_TOKEN":"PROXY_MANAGED"},"modelPicker":{"options":[{"model":"ccs-a--model"}]}});
    let direct = json!({"env":{"ANTHROPIC_BASE_URL":"https://direct.example","ANTHROPIC_API_KEY":"fixture-key"}});
    let native_login = home.path().join(".claude/.credentials.json");
    let native_bytes = br#"{"claudeAiOauth":{"accessToken":"fixture-only"}}"#;
    fs::write(&native_login, native_bytes).unwrap();
    let path = home.path().join(".claude/settings.json");
    let mut live = published.clone();
    live["hooks"] = json!({"user":true});
    fs::write(&path, serde_json::to_vec(&live).unwrap()).unwrap();
    tx.commit_project_checked(
        &project,
        &mut file.guard,
        &AppState::default(),
        true,
        &[],
        PendingTarget {
            pointer: Some("direct".into()),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            stack: Some(StackState {
                enabled: true,
                members: vec!["alpha".into()],
                keys: [("a".into(), "alpha".into())].into(),
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    let state = tx.state(AppId::Claude).unwrap();
    restore_claude(
        &mut file,
        &tx,
        Restore { project: &project, expected: &state, published: &published, direct: &direct },
        || true,
    )
    .unwrap();
    let live: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(live["env"], direct["env"]);
    assert!(live.get("modelPicker").is_none());
    assert_eq!(live["hooks"]["user"], true);
    assert_eq!(fs::read(native_login).unwrap(), native_bytes);
    let next = tx.state(AppId::Claude).unwrap();
    assert!(!next.stack.enabled);
    assert_eq!(next.stack.members, state.stack.members);
    assert_eq!(next.stack.keys, state.stack.keys);
    assert!(next.pointer == state.pointer);
    assert!(matches!(next.mode.unwrap().mode, Mode::Direct));
}

struct CodexFixture {
    home: tempfile::TempDir,
    _private: tempfile::TempDir,
    tx: ProviderTransactions,
    project: ProjectScope,
    published: Value,
    direct: Value,
}
impl CodexFixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
        let project =
            ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project"))
                .unwrap();
        let mut files = crate::providers::codex_catalog::CodexFiles::open(home.path()).unwrap();
        let config = "model='ccs-a/other'\nmodel_provider='custom'\nmodel_catalog_json='shiju-model-catalog.json'\n[model_providers.custom]\nbase_url='http://127.0.0.1:12345'\nwire_api='responses'\nexperimental_bearer_token='fixture-proxy'\n[agents]\ndefault_subagent_model='ccs-a/other'\n";
        let published = json!({"config":config,"catalog":{"models":[{"slug":"ccs-a/other"}]}});
        let direct =
            json!({"config":"model='main'\ncli_auth_credentials_store='file'\n", "auth":{}});
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
                mode: Some(ModeState {
                    mode: Mode::Proxy,
                    attached: true,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                stack: Some(StackState {
                    enabled: true,
                    members: vec!["alpha".into()],
                    keys: [("a".into(), "alpha".into())].into(),
                }),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        Self { home, _private: private, tx, project, published, direct }
    }
    fn path(&self, name: &str) -> std::path::PathBuf {
        self.home.path().join(".codex").join(name)
    }
    fn read(&self, name: &str) -> Vec<u8> {
        fs::read(self.path(name)).unwrap()
    }
    fn run(
        &self,
        project: &ProjectScope,
        expected: &AppState,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), RestoreError> {
        use crate::providers::{
            codex_catalog::CodexFiles,
            codex_login::{AuthTarget, LoginInput},
        };
        let mut files = CodexFiles::open(self.home.path()).unwrap();
        restore_codex(
            &mut files,
            &self.tx,
            Restore { project, expected, published: &self.published, direct: &self.direct },
            LoginInput {
                target: AuthTarget::Official { row_auth: &self.direct["auth"] },
                leaving_official: None,
                official_logins: &[],
                third_party_keys: &[],
                live_is_managed: false,
                now: 1_800_000_000,
            },
            None,
            unchanged,
        )
    }
}
#[test]
fn aggregation_t17_codex_restores_login_catalog_and_direct_in_one_transaction() {
    use crate::providers::codex_login::{AuthTarget, LoginInput};
    let f = CodexFixture::new();
    let login = json!({"auth_mode":"chatgpt","tokens":{"account_id":"fixture-account","access_token":"fixture-access","refresh_token":"fixture-refresh"}});
    fs::write(f.path("auth.json"), serde_json::to_vec(&login).unwrap()).unwrap();
    // Put the native login in the REAL account owner's stash before exiting.
    {
        let mut files = crate::providers::codex_catalog::CodexFiles::open(f.home.path()).unwrap();
        let prepared = crate::providers::codex_login::prepare(
            &files.guard,
            LoginInput {
                target: AuthTarget::ThirdParty { preserve: false },
                leaving_official: None,
                official_logins: &[],
                third_party_keys: &[],
                live_is_managed: false,
                now: 1_800_000_000,
            },
        )
        .unwrap();
        let state = f.tx.state(AppId::Codex).unwrap();
        f.tx.commit_project_checked(
            &f.project,
            &mut files.guard,
            &state,
            false,
            &prepared.changes(),
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
    }
    assert!(!f.path("auth.json").exists());
    let state = f.tx.state(AppId::Codex).unwrap();
    f.run(&f.project, &state, || true).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&f.read("auth.json")).unwrap(), login);
    assert!(!f.path("shiju-model-catalog.json").exists());
    let text = String::from_utf8(f.read("config.toml")).unwrap();
    let doc: Value = toml::from_str(&text).unwrap();
    assert_eq!(doc["model"], "main");
    assert!(doc.get("model_provider").is_none());
    assert!(doc.get("model_catalog_json").is_none());
    assert!(doc["agents"].get("default_subagent_model").is_none());
    assert_eq!(doc["history"]["persistence"], "save-all");
    assert!(text.contains("# user comment"));
    let after = f.tx.state(AppId::Codex).unwrap();
    assert!(!after.stack.enabled);
    assert_eq!(after.stack.members, state.stack.members);
    assert_eq!(after.stack.keys, state.stack.keys);
    assert!(after.pointer == state.pointer);
    assert!(!after.mode.unwrap().attached);
}
#[test]
fn aggregation_t17_exit_rejects_foreign_and_stale_inputs_without_writes() {
    let f = CodexFixture::new();
    let state = f.tx.state(AppId::Codex).unwrap();
    let foreign =
        ProjectScope::new(TenantScope::new("p", "other").unwrap(), ProjectId::from("project"))
            .unwrap();
    let before = f.read("config.toml");
    assert_eq!(f.run(&foreign, &state, || true), Err(RestoreError::Forbidden));
    assert_eq!(f.run(&f.project, &state, || false), Err(RestoreError::RevisionConflict));
    let mut stale = state.clone();
    stale.stack.members.push("new".into());
    assert_eq!(f.run(&f.project, &stale, || true), Err(RestoreError::RevisionConflict));
    assert_eq!(f.read("config.toml"), before);
    assert!(f.tx.state(AppId::Codex).unwrap() == state);
    assert!(f.tx.pending(AppId::Codex).unwrap().is_none());
}
#[test]
fn aggregation_t17_exit_preserves_user_owned_model_and_catalog_edits() {
    for edit_catalog in [false, true] {
        let f = CodexFixture::new();
        let state = f.tx.state(AppId::Codex).unwrap();
        if edit_catalog {
            fs::write(f.path("shiju-model-catalog.json"), br#"{"models":[{"slug":"user-new"}]}"#)
                .unwrap();
        } else {
            let changed = String::from_utf8(f.read("config.toml"))
                .unwrap()
                .replace("ccs-a/other", "user-new");
            fs::write(f.path("config.toml"), changed).unwrap();
        }
        let config = f.read("config.toml");
        let catalog = f.read("shiju-model-catalog.json");
        assert_eq!(f.run(&f.project, &state, || true), Err(RestoreError::RevisionConflict));
        assert_eq!(f.read("config.toml"), config);
        assert_eq!(f.read("shiju-model-catalog.json"), catalog);
        assert!(f.tx.state(AppId::Codex).unwrap() == state);
    }
}
#[test]
fn aggregation_t17_exit_failure_keeps_real_intent_and_scoped_recovery() {
    use crate::providers::live::{recovery::RecoveryOutcome, write_guard::WriteError};
    for phase in [Phase::Marked, Phase::Published] {
        let f = CodexFixture::new();
        let state = f.tx.state(AppId::Codex).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let result = f.run(&f.project, &state, || true);
        HOOK.with_borrow_mut(|h| *h = None);
        assert_eq!(result, Err(RestoreError::TargetFailed));
        assert!(f.tx.pending(AppId::Codex).unwrap().unwrap().published);
        assert_eq!(f.run(&f.project, &state, || true), Err(RestoreError::RecoveryPending));
        let mut files = crate::providers::codex_catalog::CodexFiles::open(f.home.path()).unwrap();
        assert_eq!(
            f.tx.recover_project(&f.project, &mut files.guard).unwrap(),
            Some(RecoveryOutcome::RolledForward)
        );
        assert!(!f.tx.state(AppId::Codex).unwrap().stack.enabled);
        assert!(!f.path("shiju-model-catalog.json").exists());
    }
}

#[test]
fn aggregation_t17_exit_rejects_keyring_and_late_revision_change() {
    use std::cell::Cell;
    let f = CodexFixture::new();
    let state = f.tx.state(AppId::Codex).unwrap();
    let original = f.read("config.toml");
    fs::write(
        f.path("config.toml"),
        format!(
            "cli_auth_credentials_store='keyring'\n{}",
            String::from_utf8(original.clone()).unwrap()
        ),
    )
    .unwrap();
    let before = f.read("config.toml");
    assert_eq!(f.run(&f.project, &state, || true), Err(RestoreError::UnsupportedTarget));
    assert_eq!(f.read("config.toml"), before);
    fs::write(f.path("config.toml"), &original).unwrap();
    let checks = Cell::new(0);
    assert_eq!(
        f.run(&f.project, &state, || {
            checks.set(checks.get() + 1);
            checks.get() < 2
        }),
        Err(RestoreError::RevisionConflict)
    );
    assert_eq!(f.read("config.toml"), original);
    assert!(f.tx.state(AppId::Codex).unwrap() == state);
    if f.tx.pending(AppId::Codex).unwrap().is_some() {
        let mut files = crate::providers::codex_catalog::CodexFiles::open(f.home.path()).unwrap();
        f.tx.recover_project(&f.project, &mut files.guard).unwrap();
    }
}
