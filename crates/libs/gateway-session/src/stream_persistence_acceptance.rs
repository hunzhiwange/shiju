//! Persistence assertions shared by the concrete host stream adapter.
use crate::stream::StreamTurnMessageIds;
use agent_project as project;
use session_ui_types as ui_models;
pub async fn persist_stream_chat_turn_creates_user_and_assistant_messages<H: crate::stream_host::StreamHost>() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_id = format!("ses_stream_test_{}", uuid::Uuid::new_v4());

    // 流式持久化依赖当前项目实例目录；测试用临时目录隔离真实用户数据。
    let (assistant_id, user_id, stored) =
        project::instance::provide(temp.path(), None, move || {
            Box::pin(async move {
                let usage = ui_models::TokenUsage {
                    input_tokens: 12,
                    output_tokens: 34,
                    cached_tokens: 5,
                    reasoning_tokens: 6,
                };
                let (assistant_id, user_id) = crate::stream::persist_stream_chat_turn::<H>(
                    &session_id,
                    "hello from user",
                    "hello from assistant",
                    Some("anthropic/claude-sonnet-4"),
                    &usage,
                    Some("stop"),
                    None,
                    &[],
                )
                .await
                .expect("stream turn should persist");
                let stored = session_engine::message::messages(&session_id, None)
                    .await
                    .expect("stored messages should load");
                Ok::<_, project::Error>((assistant_id, user_id, stored))
            })
        })
        .await
        .expect("instance context should be provided")
        .expect("stream turn should return stored messages");

    assert_eq!(stored.len(), 2);

    // 用户消息也携带模型引用，后续 UI 和审计视图需要用它重建上下文。
    let user_message = stored
        .iter()
        .find(|message| message.info.id() == user_id)
        .expect("user message should be present");
    match &user_message.info {
        session_engine::message::Info::User(info) => {
            assert_eq!(info.model.provider_id, "anthropic");
            assert_eq!(info.model.model_id, "claude-sonnet-4");
        }
        _ => panic!("expected stored user message"),
    }
    let user_text = user_message
        .parts
        .iter()
        .find_map(|part| match part {
            session_engine::message::Part::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .expect("user text part should be present");
    assert_eq!(user_text, "hello from user");

    // 助手消息必须挂到用户消息下面，避免会话树在回放时丢失父子关系。
    let assistant_message = stored
        .iter()
        .find(|message| message.info.id() == assistant_id)
        .expect("assistant message should be present");
    match &assistant_message.info {
        session_engine::message::Info::Assistant(info) => {
            assert_eq!(info.parent_id, user_id);
            assert_eq!(info.provider_id, "anthropic");
            assert_eq!(info.model_id, "claude-sonnet-4");
            assert_eq!(info.finish.as_deref(), Some("stop"));
            assert_eq!(info.tokens.input, 12);
            assert_eq!(info.tokens.output, 34);
            assert_eq!(info.tokens.reasoning, 6);
            assert_eq!(info.tokens.cache.read, 5);
        }
        _ => panic!("expected stored assistant message"),
    }
    let assistant_text = assistant_message
        .parts
        .iter()
        .find_map(|part| match part {
            session_engine::message::Part::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .expect("assistant text part should be present");
    assert_eq!(assistant_text, "hello from assistant");
}

pub async fn persist_stream_chat_turn_reuses_preallocated_message_ids<H: crate::stream_host::StreamHost>() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_id = "ses_stream_preallocated";
    // 预分配 id 来自流式响应开始阶段，落盘必须复用同一组 id 才能和前端事件对齐。
    let preallocated =
        StreamTurnMessageIds::new("msg_assistant_preallocated", "msg_user_preallocated");

    let (assistant_id, user_id, stored) =
        project::instance::provide(temp.path(), None, move || {
            let preallocated = preallocated.clone();
            Box::pin(async move {
                let usage = ui_models::TokenUsage::default();
                let (assistant_id, user_id) = crate::stream::persist_stream_chat_turn::<H>(
                    session_id,
                    "hello from user",
                    "hello from assistant",
                    Some("anthropic/claude-sonnet-4"),
                    &usage,
                    Some("stop"),
                    Some(&preallocated),
                    &[],
                )
                .await
                .expect("stream turn should persist");
                let stored = session_engine::message::messages(session_id, None)
                    .await
                    .expect("stored messages should load");
                Ok::<_, project::Error>((assistant_id, user_id, stored))
            })
        })
        .await
        .expect("instance context should be provided")
        .expect("stream turn should return stored messages");

    assert_eq!(assistant_id, "msg_assistant_preallocated");
    assert_eq!(user_id, "msg_user_preallocated");
    assert!(stored.iter().any(|message| message.info.id() == assistant_id));
    assert!(stored.iter().any(|message| message.info.id() == user_id));
}

pub async fn persist_stream_chat_turn_stores_extra_assistant_patch_parts<H: crate::stream_host::StreamHost>() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_id = "ses_stream_patch_part";
    let preallocated = StreamTurnMessageIds::new("msg_assistant_patch", "msg_user_patch");
    let patch_part = session_engine::message::Part::Patch(session_engine::message::PatchPart {
        base: session_engine::message::PartBase {
            id: "prt_patch".to_string(),
            session_id: session_id.to_string(),
            message_id: "msg_assistant_patch".to_string(),
        },
        hash: "tree_hash_before_tools".to_string(),
        files: vec!["/tmp/project/src/main.rs".to_string()],
        additions: Some(3),
        deletions: Some(1),
    });

    let stored = project::instance::provide(temp.path(), None, move || {
        let preallocated = preallocated.clone();
        let patch_part = patch_part.clone();
        Box::pin(async move {
            let usage = ui_models::TokenUsage::default();
            crate::stream::persist_stream_chat_turn::<H>(
                session_id,
                "change file",
                "changed",
                None,
                &usage,
                Some("stop"),
                Some(&preallocated),
                &[patch_part],
            )
            .await
            .expect("stream turn should persist");
            session_engine::message::messages(session_id, None).await.map_err(|err| err.to_string())
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("stored messages should load");

    let assistant = stored
        .iter()
        .find(|message| message.info.id() == "msg_assistant_patch")
        .expect("assistant message should be present");
    assert!(assistant.parts.iter().any(|part| {
        matches!(
            part,
            session_engine::message::Part::Patch(p)
                if p.hash == "tree_hash_before_tools"
                    && p.files == vec!["/tmp/project/src/main.rs".to_string()]
                    && p.additions == Some(3)
                    && p.deletions == Some(1)
        )
    }));
}
