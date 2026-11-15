#[tokio::test]
async fn child_first_response_continuation_preserves_published_title_and_history() {
    use crate::app::agent::project;
    use crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected;
    use crate::session::child_agent::{self, ChildAgentRunInput};
    use crate::session::{message, session};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_string_lossy().to_string();
    let parent_path = path.clone();
    let parent = project::instance::provide(path.clone(), None, move || {
        Box::pin(async move {
            session::create_next(session::CreateInput {
                parent_id: None,
                title: None,
                directory: parent_path,
                permission: None,
            })
            .await
        })
    })
    .await
    .unwrap()
    .unwrap();
    let mut created = child_agent::create(ChildAgentRunInput {
        parent_session_id: parent.id,
        directory: path.clone(),
        agent_key: "reviewer".into(),
        model: None,
        prompt: "first".into(),
        context: vec![],
        allowed_tools: vec![],
        default_allowed_skills: vec![],
        skill_policy: Default::default(),
        trusted_user_selected_skills: vec![],
    })
    .await
    .unwrap();
    let id = created.child_session_id.clone();
    child_agent::run_created_with_stream_executor(created.clone(), |_| async {
        Ok(GatewayChatStreamCollected {
            output: "first result".into(),
            message_id: None,
            parent_message_id: None,
        })
    })
    .await
    .unwrap();
    created.prepare_followup("second".into()).await.unwrap();
    let expected = id.clone();
    child_agent::run_created_with_stream_executor(created, move |request| async move {
        assert_eq!(request.session_id.unwrap().0, expected);
        assert!(request.messages.iter().any(|m| m["content"] == "first result"));
        Ok(GatewayChatStreamCollected {
            output: "second result".into(),
            message_id: None,
            parent_message_id: None,
        })
    })
    .await
    .unwrap();
    let (info, count) = project::instance::provide(path, None, move || {
        Box::pin(async move {
            (session::get(&id).await.unwrap(), message::messages(&id, None).await.unwrap().len())
        })
    })
    .await
    .unwrap();
    assert_eq!(count, 4);
    assert!(info.is_agent_tool());
    assert!(!info.is_first_response_pending());
    assert_eq!(info.title, "first");
}
