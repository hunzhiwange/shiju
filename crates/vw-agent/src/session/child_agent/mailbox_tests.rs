use super::*;
#[test]
fn agcollab_mailbox_preserves_order_and_isolation() {
    let c = ChildAgentCoordinator::new(128);
    c.register("root", "a", "worker", "a").unwrap();
    c.register("root", "b", "worker", "b").unwrap();
    c.register("other", "x", "worker", "x").unwrap();
    for payload in ["one", "two"] {
        assert!(c.enqueue_mail("a", "b", 1, TeamMessageKind::Message, payload.into()).is_ok());
    }
    assert!(c.enqueue_mail("a", "x", 1, TeamMessageKind::Message, "bad".into()).is_err());
    let mail = c.take_mail("b").unwrap();
    assert_eq!(mail.iter().map(|m| m.payload.as_str()).collect::<Vec<_>>(), ["one", "two"]);
    assert!(mail[0].sequence < mail[1].sequence);
    assert_eq!(mail[0].sender_session, "a");
    assert!(c.take_mail("b").unwrap().is_empty());
    assert!(c.take_mail("a").unwrap().is_empty());
}

#[test]
fn agcollab_mailbox_enforces_limits_and_closed_targets() {
    let c = ChildAgentCoordinator::new(128);
    c.register("root", "a", "worker", "a").unwrap();
    assert!(c.enqueue_mail("root", "a", 1, TeamMessageKind::Handoff, "x".repeat(65537)).is_err());
    for _ in 0..128 {
        c.enqueue_mail("root", "a", 1, TeamMessageKind::Message, "ok".into()).unwrap();
    }
    assert!(c.enqueue_mail("root", "a", 1, TeamMessageKind::Message, "full".into()).is_err());
    assert_eq!(c.take_mail("a").unwrap().len(), 128);
    c.enqueue_mail("a", "root", 1, TeamMessageKind::Handoff, "reply".into()).unwrap();
    assert_eq!(c.take_mail("root").unwrap().len(), 1);
    c.cancel("root", "a").unwrap();
    assert!(c.enqueue_mail("root", "a", 1, TeamMessageKind::Message, "closed".into()).is_err());
    assert!(
        c.enqueue_mail("root", "unknown", 1, TeamMessageKind::Message, "unknown".into()).is_err()
    );
}

#[tokio::test]
async fn agcollab_mailbox_wait_observes_messages_results_cancel_and_timeout() {
    let c = ChildAgentCoordinator::new(128);
    c.register("root", "a", "worker", "a").unwrap();
    let token = CancellationToken::new();
    assert_eq!(c.wait_mail("a", Duration::ZERO, &token).await.unwrap(), TeamActivity::TimedOut);
    let waiter = c.wait_mail("a", Duration::from_secs(5), &token);
    let sender = async {
        c.enqueue_mail("root", "a", 1, TeamMessageKind::Message, "wake".into()).unwrap();
    };
    let (outcome, _) = tokio::join!(waiter, sender);
    assert_eq!(outcome.unwrap(), TeamActivity::Message);
    c.take_mail("a").unwrap();
    token.cancel();
    assert_eq!(c.wait_mail("a", Duration::ZERO, &token).await.unwrap(), TeamActivity::Cancelled);
    c.mark_succeeded("root", "a", "done").unwrap();
    assert_eq!(
        c.wait_mail("root", Duration::ZERO, &CancellationToken::new()).await.unwrap(),
        TeamActivity::Result
    );
}

#[tokio::test(flavor = "current_thread")]
async fn agcollab_mailbox_runtime_identity_and_tenant_are_enforced() {
    use crate::app::agent::project;
    use crate::session::session;
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            let root = session::create_next(session::CreateInput {
                parent_id: None,
                title: Some("mail root".into()),
                directory: directory.clone(),
                permission: None,
            })
            .await
            .unwrap();
            let child = session::create_next(session::CreateInput {
                parent_id: Some(root.id.clone()),
                title: Some("mail child".into()),
                directory: directory.clone(),
                permission: None,
            })
            .await
            .unwrap();
            let c = ChildAgentCoordinator::new(128);
            c.register(&root.id, &child.id, "worker", "mail").unwrap();
            let sender = ToolUseContext::new(&root.id, Some(directory.clone()));
            let receiver = ToolUseContext::new(&child.id, Some(directory));
            let message = c
                .send_team_message(&sender, &child.id, 7, TeamMessageKind::Message, "data".into())
                .await
                .unwrap();
            assert_eq!(message.sender_session, root.id);
            assert_eq!(message.turn, 7);
            assert_eq!(c.consume_team_messages(&receiver).await.unwrap(), vec![message]);
            assert!(c.consume_team_messages(&receiver).await.unwrap().is_empty());
            let tenant =
                vw_api_types::id::TenantScope::new("other-platform", "other-company").unwrap();
            let result = crate::session::ui_store::with_tenant_scope(
                tenant,
                c.send_team_message(&sender, &child.id, 7, TeamMessageKind::Message, "bad".into()),
            )
            .await;
            assert!(result.unwrap_err().to_string().contains("tenant mismatch"));
            let unknown = ToolUseContext::new("unknown", None);
            assert!(
                c.send_team_message(&unknown, &child.id, 7, TeamMessageKind::Message, "bad".into())
                    .await
                    .is_err()
            );
        })
    })
    .await
    .unwrap();
}
