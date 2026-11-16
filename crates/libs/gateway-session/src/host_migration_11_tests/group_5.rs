use super::*;

#[tokio::test]
async fn stream_processing_elapsed_persists_and_reloads_explicit_request_bounds() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let session_id = format!("ses_stream_timing_{}", uuid::Uuid::new_v4());

    let stored = agent_project::instance::provide(temp.path(), None, move || {
        Box::pin(async move {
            let usage = ui_models::TokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 0,
                reasoning_tokens: 0,
            };
            let (assistant_id, _, _) = persist_stream_messages(
                9_000,
                &session_id,
                "question",
                "answer",
                Some("provider/model"),
                &usage,
                Some("stop"),
                None,
                &[],
                "gateway",
                None,
                Some(session_engine::message::AssistantTime {
                    created: 1_000,
                    completed: Some(1_250),
                    processing_elapsed_ms: Some(200),
                    llm_elapsed_ms: Some(125),
                }),
            )
            .await
            .expect("timed stream turn should persist");
            session_engine::message::get(&session_id, &assistant_id).await
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("assistant message should reload");

    let session_engine::message::Info::Assistant(info) = stored.info else {
        panic!("stored message should be assistant");
    };
    assert_eq!(info.time.created, 1_000);
    assert_eq!(info.time.completed, Some(1_250));
    assert_eq!(info.time.processing_elapsed_ms, Some(200));
    assert_eq!(info.time.llm_elapsed_ms, Some(125));
}

// 用户消息时间取请求开始时刻，避免长时间生成导致会话排序被响应完成时间扭曲。
#[tokio::test]
async fn persist_stream_turn_uses_request_start_for_user_timestamp() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let session_id = format!("ses_stream_user_time_{}", uuid::Uuid::new_v4());

    let stored = agent_project::instance::provide(temp.path(), None, move || {
        Box::pin(async move {
            let usage = ui_models::TokenUsage::default();
            persist_stream_messages(
                9_000,
                &session_id,
                "question",
                "answer",
                Some("provider/model"),
                &usage,
                Some("stop"),
                None,
                &[],
                "gateway",
                None,
                Some(session_engine::message::AssistantTime {
                    created: 1_000,
                    completed: Some(9_000),
                    processing_elapsed_ms: Some(7_500),
                    llm_elapsed_ms: Some(500),
                }),
            )
            .await
            .expect("stream turn should persist");
            session_engine::message::messages(&session_id, None).await
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("persisted messages should load");

    let user = stored
        .iter()
        .find(|message| matches!(message.info, session_engine::message::Info::User(_)))
        .expect("user message should exist");
    let session_engine::message::Info::User(info) = &user.info else {
        unreachable!("matched user info");
    };
    assert_eq!(info.time.created, 1_000);
    let session_engine::message::Part::Text(part) = &user.parts[0] else {
        panic!("user text part should exist");
    };
    assert_eq!(part.time.as_ref().map(|time| time.start), Some(1_000));
    assert_eq!(part.time.as_ref().and_then(|time| time.end), Some(1_000));
}
