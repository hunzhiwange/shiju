use super::*;
use super::super::ChildAgentCoordinator;
use crate::tools::ToolUseContext;

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
                vw_api_transport::TenantScope::new("other-platform", "other-company").unwrap();
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
