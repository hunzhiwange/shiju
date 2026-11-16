use super::*;

#[tokio::test]
async fn checkpoint_isolated_by_tenant() {
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![json!({"role":"user","content":"private"})];
    let first = vw_api_transport::TenantScope::new("platform", "company-a").unwrap();
    let second = vw_api_transport::TenantScope::new("platform", "company-b").unwrap();
    crate::session::ui_store::with_tenant_scope(
        first.clone(),
        save(&session, &source, "private summary", None),
    )
    .await
    .unwrap();
    assert_eq!(
        crate::session::ui_store::with_tenant_scope(second, restore(&session, &source))
            .await
            .unwrap(),
        source
    );
    assert_ne!(
        crate::session::ui_store::with_tenant_scope(first, restore(&session, &source))
            .await
            .unwrap(),
        source
    );
}

#[tokio::test]
async fn fork_history_copies_only_matching_checkpoints_between_directories() {
    use crate::project::instance;
    use crate::session::skill_checkpoint;
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let mut contexts = Vec::new();
    for directory in [source.path(), target.path()] {
        let (project, worktree) = crate::project::from_directory(&tenant, directory).await.unwrap();
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
        crate::skills::InvokedSkillRecord::new("fixture", &path, Some("first".into())).unwrap();
    let future =
        crate::skills::InvokedSkillRecord::new("fixture", &path, Some("future".into())).unwrap();
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
