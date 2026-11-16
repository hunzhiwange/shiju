//! Skill replay contracts over real isolated storage and an external mount port.
use agent_skills::invoked::InvokedSkillRecord;
use serde_json::Value;
use crate::skill_checkpoint::{save, restore};
use crate::compaction_checkpoint::scope_for_directory;
struct NoMountRevision;
impl crate::skill_checkpoint::MountRevision for NoMountRevision {
    async fn checkpoint_revision(_: &str) -> Result<Option<u64>, String> { Ok(None) }
}
async fn remove(session: &str, directory: &str) -> Result<(), String> {
    crate::skill_checkpoint::remove(&scope_for_directory(&session_store::ui_store::current_tenant_scope(), directory), session).await
}

use agent_storage as storage;
use serde_json::json;

#[tokio::test]
async fn codex_skills_replay_checkpoint_persists_sources_and_rejects_rewound_history() {
    if run_in_temporary_storage("codex_skills_replay_checkpoint_persists_sources_and_rejects_rewound_history") { return; }
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "BODY $ARGUMENTS").unwrap();
    let session = format!("skills-{}", uuid::Uuid::new_v4());
    let history = vec![
        json!({"role":"user","content":"start"}),
        json!({"role":"assistant","content":"skill loaded"}),
    ];
    let record = InvokedSkillRecord::new("fixture", &source, Some("local".into())).unwrap();
    save::<NoMountRevision>(&session, "main", &history, &[record.clone()]).await.unwrap();
    assert_eq!(restore::<NoMountRevision>(&session, "main", &history).await.unwrap(), vec![record.clone()]);
    assert!(save::<NoMountRevision>(&session, "main", &[], &[record.clone()]).await.is_err());
    assert_eq!(restore::<NoMountRevision>(&session, "main", &history).await.unwrap(), vec![record]);
    assert!(restore::<NoMountRevision>(&session, "main", &history[..1]).await.unwrap().is_empty());
    let mut edited = history.clone();
    edited[1]["content"] = json!("edited");
    assert!(restore::<NoMountRevision>(&session, "main", &edited).await.unwrap().is_empty());
}

#[tokio::test]
async fn codex_skills_replay_fork_copies_only_sources_and_rewind_selects_older_checkpoint() {
    if run_in_temporary_storage("codex_skills_replay_fork_copies_only_sources_and_rewind_selects_older_checkpoint") { return; }
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "BODY_ONLY_ON_DISK $ARGUMENTS").unwrap();
    let session = format!("skills-{}", uuid::Uuid::new_v4());
    let target = format!("skills-{}", uuid::Uuid::new_v4());
    let first = vec![json!({"role":"user","content":"start"})];
    let mut latest = first.clone();
    latest.push(json!({"role":"assistant","content":"invoke again"}));
    let original = InvokedSkillRecord::new("fixture", &source, Some("first".into())).unwrap();
    let changed = InvokedSkillRecord::new("fixture", &source, Some("second".into())).unwrap();
    save::<NoMountRevision>(&session, "main", &first, &[original.clone()]).await.unwrap();
    save::<NoMountRevision>(&session, "main", &latest, &[changed.clone()]).await.unwrap();
    crate::compaction_checkpoint::copy_scoped(&scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory()), &scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory()), &session, &target, None).await.unwrap();
    assert_eq!(restore::<NoMountRevision>(&target, "main", &latest).await.unwrap(), vec![changed]);
    assert_eq!(restore::<NoMountRevision>(&target, "main", &first).await.unwrap(), vec![original]);
    let scope = scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory());
    for key in storage::list(&["skill_checkpoint", &scope, &target]).await.unwrap() {
        let value: Value =
            storage::read(&key.iter().map(String::as_str).collect::<Vec<_>>()).await.unwrap();
        let serialized = value.to_string();
        assert!(!serialized.contains("BODY_ONLY_ON_DISK"));
        assert!(serialized.contains("source_hashes"));
    }
    crate::compaction_checkpoint::remove(&scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory()), &target)
        .await
        .unwrap();
    assert!(restore::<NoMountRevision>(&target, "main", &latest).await.unwrap().is_empty());
    assert_eq!(restore::<NoMountRevision>(&session, "main", &latest).await.unwrap().len(), 1);
    remove(&session, &agent_project::instance::directory()).await.unwrap();
}

#[tokio::test]
async fn codex_skills_replay_checkpoint_isolates_tenant_project_session_and_agent() {
    if run_in_temporary_storage("codex_skills_replay_checkpoint_isolates_tenant_project_session_and_agent") { return; }
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "source body").unwrap();
    let record = InvokedSkillRecord::new("fixture", &source, None).unwrap();
    let session = format!("skills-{}", uuid::Uuid::new_v4());
    let history = vec![json!({"role":"user","content":"private task"})];
    let first = vw_api_transport::TenantScope::new("platform", "first").unwrap();
    let second = vw_api_transport::TenantScope::new("platform", "second").unwrap();
    session_store::ui_store::with_tenant_scope(
        first.clone(),
        save::<NoMountRevision>(&session, "main", &history, &[record.clone()]),
    )
    .await
    .unwrap();
    assert!(
        session_store::ui_store::with_tenant_scope(second, restore::<NoMountRevision>(&session, "main", &history))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        session_store::ui_store::with_tenant_scope(
            first.clone(),
            restore::<NoMountRevision>(&session, "child", &history)
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert!(
        session_store::ui_store::with_tenant_scope(
            first.clone(),
            restore::<NoMountRevision>("different-session", "main", &history)
        )
        .await
        .unwrap()
        .is_empty()
    );
    session_store::ui_store::with_tenant_scope(first.clone(), async {
        assert_eq!(restore::<NoMountRevision>(&session, "main", &history).await.unwrap(), vec![record]);
        let context = agent_project::instance::Context {
            directory: other.path().to_string_lossy().into_owned(),
            worktree: other.path().to_string_lossy().into_owned(),
            project: agent_project::Info {
                id: "isolated-checkpoint-project".into(),
                worktree: other.path().to_string_lossy().into_owned(),
                vcs: None,
                name: None,
                icon: None,
                commands: None,
                time: agent_project::TimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: vec![],
            },
        };
        assert!(
            agent_project::instance::provide_context(context, restore::<NoMountRevision>(&session, "main", &history))
                .await
                .unwrap()
                .is_empty()
        );
        remove(&session, &agent_project::instance::directory()).await.unwrap();
    })
    .await;
}

#[test]
fn codex_skills_replay_current_source_policy_and_explicit_provenance_are_revalidated() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(
        &source,
        "---\nname: fixture\ndisable-model-invocation: true\n---\nBODY $ARGUMENTS",
    )
    .unwrap();
    let record = InvokedSkillRecord::new("fixture", &source, Some("resolved".into())).unwrap();
    assert!(record.replay().is_err());
    let record = record.with_explicit_request(true);
    assert!(record.replay().unwrap().content.contains("BODY resolved"));
    let skill = agent_skills::types::Skill {
        name: "fixture".into(),
        location: Some(source.clone()),
        ..Default::default()
    };
    let mut state = agent_skills::session_state::SkillListingSessionState::default();
    state.register_invoked("main", record);
    state.validate_invoked_sources("main", &[skill.clone()]).unwrap();
    assert!(state.validate_invoked_sources("main", &[]).is_err());
    let replacement = root.path().join("replacement.md");
    std::fs::write(&replacement, "different source").unwrap();
    assert!(
        state
            .validate_invoked_sources(
                "main",
                &[agent_skills::types::Skill { location: Some(replacement), ..skill }]
            )
            .is_err()
    );
    std::fs::write(root.path().join("SKILL.disabled"), "disabled").unwrap();
    assert!(state.restore_invoked("main").is_err());
}

#[test]
fn codex_skills_replay_crlf_flags_and_unsupported_path_conditions_are_not_bypassed() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(
        &source,
        "---\r\nname: fixture\r\ndisable-model-invocation: true\r\n---\r\nPRIVATE_BODY",
    )
    .unwrap();
    let record = InvokedSkillRecord::new("fixture", &source, None).unwrap();
    assert!(record.replay().unwrap_err().contains("禁止模型调用"));
    let record = record.with_explicit_request(true);
    assert!(record.replay().unwrap().content.contains("PRIVATE_BODY"));
    std::fs::write(&source, "---\r\nname: fixture\r\nuser-invocable: false\r\n---\r\nPRIVATE_BODY")
        .unwrap();
    assert!(record.replay().unwrap_err().contains("禁止用户调用"));
    std::fs::write(&source, "---\npaths: ['src/**']\n---\nPRIVATE_BODY").unwrap();
    assert!(record.replay().unwrap_err().contains("paths"));
}

fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_MIGRATION_25_CHILD_TEST";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        return false;
    }
    let root = tempfile::tempdir().expect("temporary storage root");
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg(format!("{}::{test}", module_path!().split_once("::").expect("crate module").1))
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_TEST, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VIBEWINDOW_TEST_HOME", root.path())
        .status()
        .expect("isolated test process");
    assert!(status.success(), "isolated test failed: {test}: {status}");
    true
}