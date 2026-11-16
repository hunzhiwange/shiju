fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_MIGRATION_23_CHECKPOINT_CHILD";
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

use serde_json::{Value,json};
fn scope() -> String { crate::compaction_checkpoint::scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory()) }
async fn save(session:&str, source:&[Value], summary:&str, retained:Option<Value>)->Result<(),String> { crate::compaction_checkpoint::save(&scope(),session,source,summary,retained).await }
async fn restore(session:&str, history:&[Value])->Result<Vec<Value>,String> { crate::compaction_checkpoint::restore(&scope(),session,history).await }
async fn copy_fork(source_directory:&str,target_directory:&str,source:&str,target:&str,history:&[Value])->Result<(),String> {
    let tenant=session_store::ui_store::current_tenant_scope();
    crate::compaction_checkpoint::copy_fork(&crate::compaction_checkpoint::scope_for_directory(&tenant,source_directory),&crate::compaction_checkpoint::scope_for_directory(&tenant,target_directory),source,target,history).await
}
mod skill_checkpoint {
    struct NoMountRevision;
    impl crate::skill_checkpoint::MountRevision for NoMountRevision { async fn checkpoint_revision(_: &str) -> Result<Option<u64>,String> { Ok(None) } }
    use super::*;
    pub async fn save(session:&str,agent:&str,history:&[Value],records:&[agent_skills::invoked::InvokedSkillRecord])->Result<(),String> { crate::skill_checkpoint::save::<NoMountRevision>(session,agent,history,records).await }
    pub async fn restore(session:&str,agent:&str,history:&[Value])->Result<Vec<agent_skills::invoked::InvokedSkillRecord>,String> { crate::skill_checkpoint::restore::<NoMountRevision>(session,agent,history).await }
}
#[tokio::test]
async fn checkpoint_isolated_by_tenant() {
    if run_in_temporary_storage("checkpoint_isolated_by_tenant") { return; }
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![json!({"role":"user","content":"private"})];
    let first = vw_api_transport::TenantScope::new("platform", "company-a").unwrap();
    let second = vw_api_transport::TenantScope::new("platform", "company-b").unwrap();
    session_store::ui_store::with_tenant_scope(
        first.clone(),
        save(&session, &source, "private summary", None),
    )
    .await
    .unwrap();
    assert_eq!(
        session_store::ui_store::with_tenant_scope(second, restore(&session, &source))
            .await
            .unwrap(),
        source
    );
    assert_ne!(
        session_store::ui_store::with_tenant_scope(first, restore(&session, &source))
            .await
            .unwrap(),
        source
    );
}

#[tokio::test]
async fn fork_history_copies_only_matching_checkpoints_between_directories() {
    if run_in_temporary_storage("fork_history_copies_only_matching_checkpoints_between_directories") { return; }
    use agent_project::instance;
    use skill_checkpoint;
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let tenant = session_store::ui_store::current_tenant_scope();
    let mut contexts = Vec::new();
    for directory in [source.path(), target.path()] {
        let (project, worktree) = agent_project::from_directory(&tenant, directory).await.unwrap();
        contexts.push(instance::Context {
            directory: directory.to_str().unwrap().into(),
            worktree,
            project,
        });
    }
    let first = vec![json!({"role":"user", "content":"first turn"})];
    let mut later = first.clone();
    later.push(json!({"role":"assistant","content":"later turn"}));
    let session = format!("source-{}", uuid::Uuid::new_v4());
    let branch = format!("branch-{}", uuid::Uuid::new_v4());
    let path = source.path().join("SKILL.md");
    std::fs::write(&path, "skill text").unwrap();
    let early =
        agent_skills::invoked::InvokedSkillRecord::new("fixture", &path, Some("first".into())).unwrap();
    let future =
        agent_skills::invoked::InvokedSkillRecord::new("fixture", &path, Some("future".into())).unwrap();
    instance::provide_context(contexts[0].clone(), async {
        save(&session, &first, "first summary", None).await.unwrap();
        save(&session, &later, "later summary", None).await.unwrap();
        skill_checkpoint::save(&session, "main", &first, std::slice::from_ref(&early))
            .await
            .unwrap();
        skill_checkpoint::save(&session, "main", &later, &[future]).await.unwrap();
    })
    .await;
    copy_fork(&contexts[0].directory, &contexts[1].directory, &session, &branch, &first)
        .await
        .unwrap();
    instance::provide_context(contexts[1].clone(), async {
        let restored = restore(&branch, &later).await.unwrap();
        assert_eq!(restored[0]["content"], "[Compaction summary]\nfirst summary");
        assert_eq!(restored.last(), later.last());
        assert_eq!(skill_checkpoint::restore(&branch, "main", &later).await.unwrap(), [early]);
    })
    .await;
}
