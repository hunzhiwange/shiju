//! Persistent AgentTool integration; only the model stream boundary is scripted.
use super::*;
use crate::session::child_agent::coordinator::ChildAgentTaskState;
use crate::session::child_agent::coordinator::mailbox::TeamMessageKind;
use crate::tools::{ToolUseContext, scope_tool_use_context};
use tokio::sync::Barrier;

#[tokio::test]
async fn agcollab_end_to_end_team_workflow() {
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    let path = directory.clone();
    let parent = crate::project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            crate::session::session::create_next(crate::session::session::CreateInput {
                parent_id: None,
                title: None,
                directory: path,
                permission: None,
            })
            .await
        })
    })
    .await
    .unwrap()
    .unwrap();
    crate::project::instance::provide(directory.clone(), None, move || Box::pin(async move {
    let coordinator = ChildAgentCoordinator::new(128);
    let barrier = Arc::new(Barrier::new(3));
    let release = Arc::new(Barrier::new(3));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (started, mut starts) = tokio::sync::mpsc::unbounded_channel();
    let executor = {
        let barrier = barrier.clone();
        let release = release.clone();
        let calls = calls.clone();
        Arc::new(move |request: vw_api_types::chat::GatewayChatStreamRequest| {
            let barrier = barrier.clone();
            let release = release.clone();
            let calls = calls.clone();
            let started = started.clone();
            Box::pin(async move {
                let call = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let id = request.session_id.unwrap().0;
                started.send(id).unwrap();
                if call < 2 {
                    barrier.wait().await;
                    release.wait().await;
                } else {
                    assert!(
                        request
                            .messages
                            .iter()
                            .any(|m| m["content"].as_str().is_some_and(|s| s.contains("fact")))
                    );
                }
                Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                    output: format!("fact {call}"),
                    message_id: None,
                    parent_message_id: None,
                })
            }) as futures_util::future::BoxFuture<'static, _>
        })
    };
    let tool = AgentTool::new(
        HashMap::from([("writer".into(), DelegateAgentConfig::default())]),
        Some(parent.id.clone()),
        directory.clone().into(),
    )
    .with_coordinator(coordinator.clone())
    .with_child_agent_stream_executor(executor);
    let context = Arc::new(ToolUseContext::new(&parent.id, Some(directory.clone())));
    let mut ids = Vec::new();
    for prompt in ["research alpha", "research beta"] {
        let result = scope_tool_use_context(
            context.clone(),
            tool.execute(json!({"agent":"writer","prompt":prompt})),
        )
        .await
        .unwrap();
        let body: Value = serde_json::from_str(&result.output).unwrap();
        ids.push(body["child_session_id"].as_str().unwrap().to_string());
    }
    let overlap = tokio::time::timeout(Duration::from_secs(10), barrier.wait()).await;
    assert!(overlap.is_ok(), "{:?}", coordinator.snapshots(&parent.id, &ids));
    assert_ne!(starts.recv().await.unwrap(), starts.recv().await.unwrap());
    // Both children are blocked inside the model boundary while the parent continues.
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    for (sender, target) in [(&parent.id, &ids[0]), (&ids[0], &parent.id), (&ids[0], &ids[1])] {
        let sender_context = ToolUseContext::new(sender, None);
        coordinator
            .send_team_message(
                &sender_context,
                target,
                1,
                TeamMessageKind::Message,
                "evidence".into(),
            )
            .await
            .unwrap();
        let mut messages = Vec::new();
        assert!(
            coordinator
                .deliver_model_events(&ToolUseContext::new(target, None), &mut messages)
                .unwrap()
        );
        assert!(messages[0]["content"].as_str().unwrap().contains(sender));
        assert!(
            !coordinator
                .deliver_model_events(&ToolUseContext::new(target, None), &mut messages)
                .unwrap()
        );
    }
    assert!(
        coordinator
            .send_team_message(
                &context,
                "outside-team",
                1,
                TeamMessageKind::Message,
                "denied".into()
            )
            .await
            .is_err()
    );
    release.wait().await;
    assert!(
        !coordinator
            .wait(&parent.id, &ids, WaitCondition::All, Duration::from_secs(10))
            .await
            .unwrap()
            .timed_out
    );
    let mut history = Vec::new();
    assert!(coordinator.deliver_model_events(&context, &mut history).unwrap());
    assert_eq!(history.len(), 2);
    scope_tool_use_context(
        context.clone(),
        tool.execute(json!({"action":"followup_task","target":ids[0],"message":"refine evidence"})),
    )
    .await
    .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(10), starts.recv()).await.unwrap().unwrap(),
        ids[0]
    );
    let outcome = coordinator
        .wait(&parent.id, &ids, WaitCondition::All, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(!outcome.timed_out);
    assert!(outcome.tasks.iter().any(|s| s.child_session_id == ids[0] && s.turn == 2));
    assert!(coordinator.deliver_model_events(&context, &mut history).unwrap());
    assert_eq!(history.len(), 3);
    assert!(!coordinator.deliver_model_events(&context, &mut history).unwrap());
    assert!(history.last().unwrap()["content"].as_str().unwrap().contains("fact 2"));
    for id in ids {
        coordinator.cancel(&parent.id, &id).unwrap();
    }
    })).await.unwrap();
}

#[tokio::test]
async fn agcollab_end_to_end_cancel_and_budget() {
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().to_string();
    let path = directory.clone();
    let parent = crate::project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            crate::session::session::create_next(crate::session::session::CreateInput {
                parent_id: None,
                title: None,
                directory: path,
                permission: None,
            })
            .await
        })
    })
    .await
    .unwrap()
    .unwrap();
    let coordinator = ChildAgentCoordinator::new(128);
    let make_tool = |owner: String| {
        AgentTool::new(
            HashMap::from([("writer".into(), DelegateAgentConfig::default())]),
            Some(owner),
            directory.clone().into(),
        )
        .with_coordinator(coordinator.clone())
        .with_child_agent_stream_executor(Arc::new(|_| Box::pin(std::future::pending())))
    };
    let first = make_tool(parent.id.clone());
    let rebuilt = make_tool(parent.id.clone());
    let mut ids = Vec::new();
    for index in 0..4 {
        let result = first
            .execute(json!({"agent":"writer","prompt":format!("work {index}")}))
            .await
            .unwrap();
        assert!(result.success);
        let body: Value = serde_json::from_str(&result.output).unwrap();
        ids.push(body["child_session_id"].as_str().unwrap().to_string());
    }
    let overflow = rebuilt.execute(json!({"agent":"writer","prompt":"overflow"})).await.unwrap();
    assert!(!overflow.success);
    let nested = make_tool(ids[0].clone());
    assert!(
        !nested
            .execute(json!({"agent":"writer","prompt":"nested overflow"}))
            .await
            .unwrap()
            .success
    );
    first.execute(json!({"action":"stop","child_session_ids":ids})).await.unwrap();
    let outcome = coordinator
        .wait(&parent.id, &ids, WaitCondition::All, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(!outcome.timed_out);
    assert!(outcome.tasks.iter().all(|s| s.state == ChildAgentTaskState::Cancelled));
    assert!(coordinator.followup_task(&parent.id, &ids[0], "cannot revive".into()).is_err());
    let capacity = coordinator.team_capacity(&parent.id);
    let permits = tokio::time::timeout(Duration::from_secs(10), capacity.acquire_many_owned(4))
        .await
        .unwrap()
        .unwrap();
    drop(permits);
}
