use super::*;
use crate::tools::context::{ToolUseContext, scope_tool_use_context};

#[tokio::test]
async fn agcollab_tool_actions_reach_shared_runtime() {
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    crate::app::agent::project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            let root = crate::session::session::create_next(crate::session::session::CreateInput {
                parent_id: None,
                title: None,
                directory: directory.clone(),
                permission: None,
            })
            .await
            .unwrap();
            let child =
                crate::session::session::create_next(crate::session::session::CreateInput {
                    parent_id: Some(root.id.clone()),
                    title: None,
                    directory: directory.clone(),
                    permission: None,
                })
                .await
                .unwrap();
            let coordinator = ChildAgentCoordinator::new(128);
            coordinator.register(&root.id, &child.id, "writer", "work").unwrap();
            let mut continuation = coordinator.attach_continuation(&child.id);
            let tool =
                AgentTool::new(HashMap::new(), Some(root.id.clone()), directory.clone().into())
                    .with_coordinator(coordinator.clone());
            let context = ToolUseContext::new(&root.id, Some(directory.clone()));
            scope_tool_use_context(context.into(), async {
                let result = tool
                    .execute(json!({"action":"send_message","target":child.id,"message":"hello"}))
                    .await
                    .unwrap();
                assert!(result.success);
                let receiver = ToolUseContext::new(&child.id, Some(directory));
                assert_eq!(
                    coordinator.consume_team_messages(&receiver).await.unwrap()[0].payload,
                    "hello"
                );
                let result = tool
                    .execute(json!({"action":"followup_task","target":child.id,"message":"next"}))
                    .await
                    .unwrap();
                assert!(result.success);
                assert_eq!(continuation.recv().await.unwrap(), "next");
                let result = tool.execute(json!({"action":"list_agents"})).await.unwrap();
                let body: Value = serde_json::from_str(&result.output).unwrap();
                assert_eq!(body["tasks"][0]["child_session_id"], child.id);
                assert!(body["tasks"][0]["turn"].as_u64().is_some());
                let result = tool
                    .execute(json!({"action":"wait","condition":"activity","timeout_ms":0}))
                    .await
                    .unwrap();
                assert_eq!(
                    serde_json::from_str::<Value>(&result.output).unwrap()["activity"],
                    "timed_out"
                );
                coordinator
                    .send_team_message(
                        &receiver,
                        &root.id,
                        0,
                        crate::session::child_agent::coordinator::mailbox::TeamMessageKind::Message,
                        "reply".into(),
                    )
                    .await
                    .unwrap();
                let result = tool
                    .execute(json!({"action":"wait","condition":"activity","timeout_ms":0}))
                    .await
                    .unwrap();
                assert_eq!(
                    serde_json::from_str::<Value>(&result.output).unwrap()["activity"],
                    "message"
                );
                coordinator.cancel(&root.id, &child.id).unwrap();
                assert!(
                    tool.execute(
                        json!({"action":"send_message","target":child.id,"message":"closed"})
                    )
                    .await
                    .is_err()
                );
                assert!(
                    tool.execute(
                        json!({"action":"followup_task","target":child.id,"message":"closed"})
                    )
                    .await
                    .is_err()
                );
            })
            .await;
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn agcollab_tool_rejects_invalid_action_fields() {
    let tool = AgentTool::new(HashMap::new(), Some("root".into()), ".".into());
    for args in [
        json!({"action":"list_agents","unknown":true}),
        json!({"action":"send_message","target":"","message":"hello"}),
        json!({"action":"followup_task","target":"child"}),
        json!({"action":"status","message":"ignored"}),
        json!({"action":12}),
    ] {
        assert!(tool.execute(args).await.is_err());
    }
}
