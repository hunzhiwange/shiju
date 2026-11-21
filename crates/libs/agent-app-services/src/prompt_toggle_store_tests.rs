use super::{PromptEntry, PromptScope, PromptsStore};
use agent_database::GlobalDatabase;
use std::{collections::BTreeMap, fs, os::unix::fs::DirBuilderExt};
use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};
use vw_api_app_identity::AppId;
use vw_api_transport::{LOCAL_HOST_ID, ProjectScope, local_project_id};

#[test]
fn first_local_activation_creates_missing_codex_prompt_after_snapshot_checks() {
    let home = tempfile::tempdir().unwrap();
    let directory = home.path().join(".codex");
    fs::create_dir(&directory).unwrap();
    let target = directory.join("AGENTS.md");
    let private = home.path().join("private");
    fs::DirBuilder::new().mode(0o700).create(&private).unwrap();
    let tx =
        ProviderTransactions::new(&private, ExecutionHostId::new(LOCAL_HOST_ID).unwrap()).unwrap();
    let project = ProjectScope::for_default_tenant(local_project_id()).unwrap();
    let other = ProjectScope::for_default_tenant("other".into()).unwrap();
    let scope = PromptScope { project: &project, host: LOCAL_HOST_ID, app: AppId::Codex };
    let store = PromptsStore::new(GlobalDatabase::for_home(home.path()).unwrap()).unwrap();
    let saved = store
        .create(
            scope,
            0,
            PromptEntry {
                id: "rule".into(),
                name: "Rule".into(),
                content: "Local instructions".into(),
                description: None,
                created_at: None,
            },
        )
        .unwrap();
    let activate = |scope, revision| {
        store.toggle_native(
            scope,
            revision,
            "rule",
            true,
            None,
            home.path(),
            None,
            &BTreeMap::new(),
            &tx,
            || Ok(()),
        )
    };

    assert!(matches!(
        activate(PromptScope { project: &other, ..scope }, saved.revision),
        Err(error) if error.to_string() == "prompt target unbound"
    ));
    assert!(activate(scope, saved.revision - 1).is_err());
    fs::write(&target, b"external").unwrap();
    assert!(activate(scope, saved.revision).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"external");
    assert!(tx.state(AppId::Codex).unwrap().project_scope.is_none());
    fs::remove_file(&target).unwrap();

    let active = activate(scope, saved.revision).unwrap();
    assert_eq!(active.active_id.as_deref(), Some("rule"));
    assert_eq!(active.revision, saved.revision + 1);
    assert_eq!(fs::read_to_string(&target).unwrap(), "Local instructions");
    assert!(tx.state(AppId::Codex).unwrap().require_project(&project).is_ok());
}
