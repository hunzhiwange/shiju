fn run_in_temporary_storage(test: &str) -> bool {
    const CHILD_TEST: &str = "SHIJU_HOST_MIGRATION_23_CHILD_TEST";
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


use serde_json::json;
#[tokio::test]
async fn checkpoint_restores_model_context_without_mutating_source() {
    if run_in_temporary_storage("checkpoint_restores_model_context_without_mutating_source") { return; }
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![
        json!({"role":"user","content":"old task"}),
        json!({"role":"assistant","content":"old details"}),
    ];
    let mut history = source.clone();
    history.push(json!({"role":"user","content":"continue"}));
    crate::compaction_checkpoint::save("migration-23", &session, &source, "remember decisions", None).await.unwrap();
    let restored = crate::compaction_checkpoint::restore("migration-23", &session, &history).await.unwrap();
    assert_eq!(restored[0]["content"], "[Compaction summary]\nremember decisions");
    assert_eq!(restored.last(), history.last());
    assert!(!restored.iter().any(|item| item["content"] == "old details"));
    assert_eq!(history[1]["content"], "old details");
    let mut edited = history.clone();
    edited[0]["content"] = json!("different task");
    assert_eq!(crate::compaction_checkpoint::restore("migration-23", &session, &edited).await.unwrap(), edited);
    assert_eq!(crate::compaction_checkpoint::restore("migration-23", &session, &source[..1]).await.unwrap(), source[..1]);
}

#[tokio::test]
async fn checkpoint_failed_save_keeps_previous_checkpoint() {
    if run_in_temporary_storage("checkpoint_failed_save_keeps_previous_checkpoint") { return; }
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![
        json!({"role":"user","content":"task"}),
        json!({"role":"assistant","content":"details"}),
    ];
    crate::compaction_checkpoint::save("migration-23", &session, &source, "previous", None).await.unwrap();
    assert!(crate::compaction_checkpoint::save("migration-23", &session, &source, " ", None).await.is_err());
    assert_eq!(
        crate::compaction_checkpoint::restore("migration-23", &session, &source).await.unwrap()[0]["content"],
        "[Compaction summary]\nprevious"
    );
    let mut changed = source.clone();
    changed.push(json!({"role":"assistant","content":"new detail"}));
    crate::compaction_checkpoint::save("migration-23", &session, &changed, "newer", None).await.unwrap();
    assert_eq!(
        crate::compaction_checkpoint::restore("migration-23", &session, &source).await.unwrap()[0]["content"],
        "[Compaction summary]\nprevious"
    );
}

#[tokio::test]
async fn checkpoint_fork_uses_matching_prefix_and_removal_keeps_source() {
    if run_in_temporary_storage("checkpoint_fork_uses_matching_prefix_and_removal_keeps_source") { return; }
    let source_session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let fork_session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let history = vec![
        json!({"role":"user","content":"task"}),
        json!({"role":"assistant","content":"details"}),
    ];
    crate::compaction_checkpoint::save("migration-23", &source_session, &history, "summary", None).await.unwrap();
    crate::compaction_checkpoint::copy_scoped("migration-23", "migration-23", &source_session, &fork_session, None).await.unwrap();
    assert_eq!(
        crate::compaction_checkpoint::restore("migration-23", &source_session, &history).await.unwrap(),
        crate::compaction_checkpoint::restore("migration-23", &fork_session, &history).await.unwrap()
    );
    assert_eq!(crate::compaction_checkpoint::restore("migration-23", &fork_session, &history[..1]).await.unwrap(), history[..1]);
    crate::compaction_checkpoint::remove("migration-23", &fork_session).await.unwrap();
    assert_eq!(crate::compaction_checkpoint::restore("migration-23", &fork_session, &history).await.unwrap(), history);
    assert_ne!(crate::compaction_checkpoint::restore("migration-23", &source_session, &history).await.unwrap(), history);
}

#[tokio::test]
async fn checkpoint_rejects_changed_tool_arguments() {
    if run_in_temporary_storage("checkpoint_rejects_changed_tool_arguments") { return; }
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![
        json!({"role":"assistant","tool_calls":[{"id":"call","function":{"name":"read","arguments":"old"}}]}),
    ];
    crate::compaction_checkpoint::save("migration-23", &session, &source, "summary", None).await.unwrap();
    let mut changed = source.clone();
    changed[0]["tool_calls"][0]["function"]["arguments"] = json!("different");
    assert_eq!(crate::compaction_checkpoint::restore("migration-23", &session, &changed).await.unwrap(), changed);
}
