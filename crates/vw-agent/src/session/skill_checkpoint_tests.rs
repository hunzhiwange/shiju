use super::*;
use serde_json::json;

#[tokio::test]
async fn codex_skills_replay_checkpoint_persists_sources_and_rejects_rewound_history() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "BODY $ARGUMENTS").unwrap();
    let session = format!("skills-{}", uuid::Uuid::new_v4());
    let history = vec![
        json!({"role":"user","content":"start"}),
        json!({"role":"assistant","content":"skill loaded"}),
    ];
    let record = InvokedSkillRecord::new("fixture", &source, Some("local".into())).unwrap();
    save(&session, "main", &history, &[record.clone()]).await.unwrap();
    assert_eq!(restore(&session, "main", &history).await.unwrap(), vec![record.clone()]);
    assert!(save(&session, "main", &[], &[record.clone()]).await.is_err());
    assert_eq!(restore(&session, "main", &history).await.unwrap(), vec![record]);
    assert!(restore(&session, "main", &history[..1]).await.unwrap().is_empty());
    let mut edited = history.clone();
    edited[1]["content"] = json!("edited");
    assert!(restore(&session, "main", &edited).await.unwrap().is_empty());
}

#[tokio::test]
async fn codex_skills_replay_fork_copies_only_sources_and_rewind_selects_older_checkpoint() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
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
    save(&session, "main", &first, &[original.clone()]).await.unwrap();
    save(&session, "main", &latest, &[changed.clone()]).await.unwrap();
    super::super::compaction_checkpoint::copy(&session, &target).await.unwrap();
    assert_eq!(restore(&target, "main", &latest).await.unwrap(), vec![changed]);
    assert_eq!(restore(&target, "main", &first).await.unwrap(), vec![original]);
    let scope = scope_for_directory(&crate::project::instance::directory());
    for key in storage::list(&["skill_checkpoint", &scope, &target]).await.unwrap() {
        let value: Value =
            storage::read(&key.iter().map(String::as_str).collect::<Vec<_>>()).await.unwrap();
        let serialized = value.to_string();
        assert!(!serialized.contains("BODY_ONLY_ON_DISK"));
        assert!(serialized.contains("source_hashes"));
    }
    super::super::compaction_checkpoint::remove(&target, &crate::project::instance::directory())
        .await
        .unwrap();
    assert!(restore(&target, "main", &latest).await.unwrap().is_empty());
    assert_eq!(restore(&session, "main", &latest).await.unwrap().len(), 1);
    remove(&session, &crate::project::instance::directory()).await.unwrap();
}

#[tokio::test]
async fn codex_skills_replay_checkpoint_isolates_tenant_project_session_and_agent() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "source body").unwrap();
    let record = InvokedSkillRecord::new("fixture", &source, None).unwrap();
    let session = format!("skills-{}", uuid::Uuid::new_v4());
    let history = vec![json!({"role":"user","content":"private task"})];
    let first = vw_api_types::id::TenantScope::new("platform", "first").unwrap();
    let second = vw_api_types::id::TenantScope::new("platform", "second").unwrap();
    crate::session::ui_store::with_tenant_scope(
        first.clone(),
        save(&session, "main", &history, &[record.clone()]),
    )
    .await
    .unwrap();
    assert!(
        crate::session::ui_store::with_tenant_scope(second, restore(&session, "main", &history))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        crate::session::ui_store::with_tenant_scope(
            first.clone(),
            restore(&session, "child", &history)
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert!(
        crate::session::ui_store::with_tenant_scope(
            first.clone(),
            restore("different-session", "main", &history)
        )
        .await
        .unwrap()
        .is_empty()
    );
    crate::session::ui_store::with_tenant_scope(first.clone(), async {
        assert_eq!(restore(&session, "main", &history).await.unwrap(), vec![record]);
        let context = crate::project::instance::Context {
            directory: other.path().to_string_lossy().into_owned(),
            worktree: other.path().to_string_lossy().into_owned(),
            project: crate::project::Info {
                id: "isolated-checkpoint-project".into(),
                worktree: other.path().to_string_lossy().into_owned(),
                vcs: None,
                name: None,
                icon: None,
                commands: None,
                time: crate::project::TimeInfo { created: 0, updated: 0, initialized: None },
                sandboxes: vec![],
            },
        };
        assert!(
            crate::project::instance::provide_context(context, restore(&session, "main", &history))
                .await
                .unwrap()
                .is_empty()
        );
        remove(&session, &crate::project::instance::directory()).await.unwrap();
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
    let skill = crate::skills::Skill {
        name: "fixture".into(),
        location: Some(source.clone()),
        ..Default::default()
    };
    let mut state = crate::skills::SkillListingSessionState::default();
    state.register_invoked("main", record);
    state.validate_invoked_sources("main", &[skill.clone()]).unwrap();
    assert!(state.validate_invoked_sources("main", &[]).is_err());
    let replacement = root.path().join("replacement.md");
    std::fs::write(&replacement, "different source").unwrap();
    assert!(
        state
            .validate_invoked_sources(
                "main",
                &[crate::skills::Skill { location: Some(replacement), ..skill }]
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
