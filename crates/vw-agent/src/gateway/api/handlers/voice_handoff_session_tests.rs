//! Uses the real session producer and approval manager; replaces only paid LLM output.
use super::voice_handoff::*;
use crate::session::{llm::StreamEvent, processor, session};
use serde_json::json;
use vw_api_types::voice::*;

async fn binding(directory: String) -> Binding {
    let tenant = crate::session::ui_store::current_tenant_scope();
    crate::project::from_directory(&tenant, &directory).await.unwrap();
    let dir = directory.clone();
    let info = crate::gateway::instance::with_instance(directory.clone(), move || {
        Box::pin(async move {
            session::create_next(session::CreateInput { directory: dir, ..Default::default() })
                .await
                .map_err(|e| crate::gateway::ApiError::internal(e.to_string()))
        })
    })
    .await
    .unwrap();
    Binding { tenant, directory, chat_session: info.id.try_into().unwrap() }
}
fn call() -> HandoffCall {
    HandoffCall {
        call_id: "call1".to_owned().try_into().unwrap(),
        item_id: "tool1".to_owned().try_into().unwrap(),
        response_id: "r1".to_owned().try_into().unwrap(),
        generation: VoiceCounter::FIRST,
        instruction: "按要求处理项目工作".into(),
    }
}
fn answer(text: &str) {
    processor::enqueue_test_llm_stream(vec![
        StreamEvent::Delta(text.into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);
}
#[tokio::test(flavor = "multi_thread")]
async fn voice_task_12_existing_session_duplicate_uses_real_producer_once() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let _streams = processor::test_llm_stream_guard();
    crate::config::update(json!({"agents":{"main":{"provider":"model-recovery","model":"original","allowed_tools":["file_read"],"defaults":{"full_access":false}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let binding =
        binding(workspace.path().canonicalize().unwrap().to_string_lossy().into_owned()).await;
    let session = binding.chat_session.as_ref().to_owned();
    answer("前文答案");
    super::session::stream::collect_gateway_chat_stream(
        binding.directory.clone(),
        vw_api_types::chat::GatewayChatStreamRequest {
            session_id: Some(session.clone().into()),
            messages: vec![json!({"role":"user","content":"前文需求"})],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    processor::take_test_llm_messages(&session);
    answer("工作已完成");
    let mut handoffs = Handoffs::new(binding);
    handoffs.submit(call(), VoiceCounter::FIRST).unwrap();
    handoffs.submit(call(), VoiceCounter::FIRST).unwrap();
    let final_update = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let update = handoffs.updates.recv().await.unwrap();
            if matches!(update.status, HandoffStatus::Completed | HandoffStatus::Failed) {
                break update;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(final_update.status, HandoffStatus::Completed, "{}", final_update.text);
    assert!(final_update.text.contains("工作已完成"));
    let requests = processor::take_test_llm_messages(&session);
    assert_eq!(requests.len(), 1, "duplicate tool final must run the real session producer once");
    assert!(
        requests[0].messages.iter().any(|m| m["role"] == "user"
            && m["content"].as_str().is_some_and(|v| v.contains("前文需求"))),
        "existing user history must remain user content"
    );
    assert!(
        requests[0].messages.iter().any(|m| m["role"] == "assistant"
            && m["content"].as_str().is_some_and(|v| v.contains("前文答案"))),
        "existing assistant history must be preserved"
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn voice_task_12_existing_session_desktop_approval_rejection_prevents_write() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let _streams = processor::test_llm_stream_guard();
    crate::config::update(json!({"agents":{"main":{"provider":"model-recovery","model":"original","allowed_tools":["file_write"],"defaults":{"full_access":false},"permission":{"*":"ask"}}},"autonomy":{"level":"supervised","always_ask":["file_write"]},"providers":{"model-recovery":{"models":{"original":{"tool_call":true}}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let binding =
        binding(workspace.path().canonicalize().unwrap().to_string_lossy().into_owned()).await;
    // Exercise handoff from an existing conversation, with persisted history/model binding.
    let session = binding.chat_session.as_ref().to_owned();
    answer("会话已准备好");
    super::session::stream::collect_gateway_chat_stream(
        binding.directory.clone(),
        vw_api_types::chat::GatewayChatStreamRequest {
            session_id: Some(session.clone().into()),
            messages: vec![json!({"role":"user","content":"准备语音交接验收"})],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let initial = processor::take_test_llm_messages(&session);
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].options["full_access"], false, "the original session must use approvals");
    assert!(
        initial[0].tool_specs.iter().any(|tool| tool.name == "file_write"),
        "fixture must offer the real write tool before a scripted call"
    );
    let target = workspace.path().join("must-not-exist.txt");
    let manager = crate::gateway::instance::with_instance(binding.directory.clone(), || {
        Box::pin(async {
            Ok::<_, crate::gateway::ApiError>(
                crate::gateway::approval_state::approval_manager_for_current_instance().await,
            )
        })
    })
    .await
    .unwrap();
    assert!(manager.needs_approval("file_write"), "fixture must require desktop approval");
    let session = binding.chat_session.as_ref().to_owned();
    processor::enqueue_test_llm_stream(vec![
        StreamEvent::ToolCalls(vec![crate::session::llm::ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "write1".into(),
            name: "file_write".into(),
            arguments: json!({"path":"must-not-exist.txt","content":"forbidden"}).to_string(),
        }]),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("tool_calls".into()) },
    ]);
    answer("模型声称写入已完成");
    let mut handoffs = Handoffs::new(binding);
    handoffs.submit(call(), VoiceCounter::FIRST).unwrap();
    let final_update = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut rejected = false;
        let mut saw_waiting = false;
        loop {
            let update = handoffs.updates.recv().await.unwrap();
            if update.status == HandoffStatus::AwaitingApproval {
                saw_waiting = true;
                for request in manager.list_non_cli_pending_requests(None, None, Some(&session)) {
                    manager.reply_to_non_cli_pending_once(&request, false).unwrap();
                    rejected = true;
                }
            }
            if matches!(update.status, HandoffStatus::Completed | HandoffStatus::Failed) {
                assert!(
                    saw_waiting && rejected,
                    "must reject a real request from the desktop approval queue: status={:?}, text={}", update.status, update.text
                );
                break update;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(final_update.status, HandoffStatus::Failed);
    assert!(final_update.text.contains("拒绝"));
    assert!(!target.exists(), "denied work must not modify the filesystem");
}
