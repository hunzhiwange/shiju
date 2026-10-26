use super::*;

#[tokio::test]
async fn checkpoint_restores_model_context_without_mutating_source() {
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![
        json!({"role":"user","content":"old task"}),
        json!({"role":"assistant","content":"old details"}),
    ];
    let mut history = source.clone();
    history.push(json!({"role":"user","content":"continue"}));
    save(&session, &source, "remember decisions", None).await.unwrap();
    let restored = restore(&session, &history).await.unwrap();
    assert_eq!(restored[0]["content"], "[Compaction summary]\nremember decisions");
    assert_eq!(restored.last(), history.last());
    assert!(!restored.iter().any(|item| item["content"] == "old details"));
    assert_eq!(history[1]["content"], "old details");
    let mut edited = history.clone();
    edited[0]["content"] = json!("different task");
    assert_eq!(restore(&session, &edited).await.unwrap(), edited);
    assert_eq!(restore(&session, &source[..1]).await.unwrap(), source[..1]);
}

#[tokio::test]
async fn checkpoint_failed_save_keeps_previous_checkpoint() {
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![
        json!({"role":"user","content":"task"}),
        json!({"role":"assistant","content":"details"}),
    ];
    save(&session, &source, "previous", None).await.unwrap();
    assert!(save(&session, &source, " ", None).await.is_err());
    assert_eq!(
        restore(&session, &source).await.unwrap()[0]["content"],
        "[Compaction summary]\nprevious"
    );
    let mut changed = source.clone();
    changed.push(json!({"role":"assistant","content":"new detail"}));
    save(&session, &changed, "newer", None).await.unwrap();
    assert_eq!(
        restore(&session, &source).await.unwrap()[0]["content"],
        "[Compaction summary]\nprevious"
    );
}

#[tokio::test]
async fn checkpoint_fork_uses_matching_prefix_and_removal_keeps_source() {
    let source_session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let fork_session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let history = vec![
        json!({"role":"user","content":"task"}),
        json!({"role":"assistant","content":"details"}),
    ];
    save(&source_session, &history, "summary", None).await.unwrap();
    copy(&source_session, &fork_session).await.unwrap();
    assert_eq!(
        restore(&source_session, &history).await.unwrap(),
        restore(&fork_session, &history).await.unwrap()
    );
    assert_eq!(restore(&fork_session, &history[..1]).await.unwrap(), history[..1]);
    remove(&fork_session, &crate::project::instance::directory()).await.unwrap();
    assert_eq!(restore(&fork_session, &history).await.unwrap(), history);
    assert_ne!(restore(&source_session, &history).await.unwrap(), history);
}

#[tokio::test]
async fn checkpoint_rejects_changed_tool_arguments() {
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![
        json!({"role":"assistant","tool_calls":[{"id":"call","function":{"name":"read","arguments":"old"}}]}),
    ];
    save(&session, &source, "summary", None).await.unwrap();
    let mut changed = source.clone();
    changed[0]["tool_calls"][0]["function"]["arguments"] = json!("different");
    assert_eq!(restore(&session, &changed).await.unwrap(), changed);
}

#[tokio::test]
async fn checkpoint_isolated_by_tenant() {
    let session = format!("checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![json!({"role":"user","content":"private"})];
    let first = vw_api_types::id::TenantScope::new("platform", "company-a").unwrap();
    let second = vw_api_types::id::TenantScope::new("platform", "company-b").unwrap();
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
