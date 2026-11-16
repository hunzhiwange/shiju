//! 历史命令经过真实入站入口、项目和会话存储；只有外部网络被替换。
use super::runtime_command::session_commands_tests::{
    lifecycle_dispatch, lifecycle_fixture, lifecycle_id,
};
use super::*;
use crate::session::message;

use channel_runtime::command_history_contracts::seed;

async fn fixture()
-> (tempfile::TempDir, ChannelRuntimeContext, traits::ChannelMessage, String, Vec<String>) {
    let (temp, ctx, mut msg) = lifecycle_fixture().await;
    msg.channel = "lark".into();
    let mut ctx = ctx;
    ctx.message_snapshot = None;
    ctx.message_snapshot = Some(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap());
    assert!(lifecycle_dispatch(&ctx, &msg, "/new").await.contains("已创建"));
    let id = lifecycle_id(&ctx);
    let ids = seed(&ctx, &id).await;
    (temp, ctx, msg, id, ids)
}

#[tokio::test]
async fn channel_session_history_lists_real_anchors_without_mutation() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_lists_real_anchors_without_mutation(ctx, msg, id, ids, dispatch_contract).await;
}

#[tokio::test]
async fn channel_session_history_fork_switches_and_preserves_source() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_fork_switches_and_preserves_source(ctx, msg, id, ids, dispatch_contract).await;
}

#[tokio::test]
async fn channel_session_history_foreign_anchors_preserve_cache_and_binding() {
    let (_temp, ctx, msg, id, _) = fixture().await;
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    ctx.conversation_histories.lock().unwrap().insert(key.clone(), vec![ChatMessage::user("keep")]);
    let (_foreign_temp, _foreign_ctx, _foreign_msg, _foreign_id, foreign_ids) = fixture().await;
    for name in ["fork", "compact", "reset"] {
        let response = lifecycle_dispatch(&ctx, &msg, &format!("/{name} {}", foreign_ids[0])).await;
        assert!(response.contains("失败"), "{response}");
        assert_eq!(lifecycle_id(&ctx), id);
        assert_eq!(ctx.conversation_histories.lock().unwrap()[&key][0].content, "keep");
        assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);
    }
}

#[tokio::test]
async fn channel_session_history_reset_requires_confirmation_and_reloads_cache() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_reset_requires_confirmation_and_reloads_cache(ctx, msg, id, ids, dispatch_contract).await;
}

#[tokio::test]
async fn channel_session_history_stale_confirmation_is_rejected() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_stale_confirmation_is_rejected(ctx, msg, id, ids, dispatch_contract).await;
}

#[tokio::test]
async fn channel_session_history_cross_scope_and_busy_are_rejected() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_cross_scope_and_busy_are_rejected(ctx, msg, id, ids, dispatch_contract).await;
}

struct ModelServer {
    task: tokio::task::JoinHandle<()>,
    key: String,
}
impl ModelServer {
    fn abort(&self) {
        self.task.abort();
    }
}
impl Drop for ModelServer {
    fn drop(&mut self) {
        self.task.abort();
        agent_env::remove(&self.key);
    }
}

// 提供项目模型配置；摘要响应由共享 runner 的模型脚本边界提供。
async fn model_server(
    ctx: &ChannelRuntimeContext,
    summary: &'static str,
) -> (ModelServer, Arc<Mutex<Vec<serde_json::Value>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    let app = axum::Router::new().route("/v1/chat/completions", axum::routing::post(move |axum::Json(body): axum::Json<serde_json::Value>| {
        let recorded = recorded.clone();
        async move {
            recorded.lock().unwrap().push(body);
            let delta = serde_json::json!({"id":"fixture", "object":"chat.completion.chunk", "created":1, "model":"fixture", "choices":[{"index":0,"delta":{"content":summary},"finish_reason":null}]});
            let done = serde_json::json!({"id":"fixture", "object":"chat.completion.chunk", "created":1, "model":"fixture", "choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
            ([("content-type", "text/event-stream")], format!("data: {delta}\n\ndata: {done}\n\ndata: [DONE]\n\n"))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let key = format!("VW_HISTORY_FIXTURE_{}", snapshot.scope.project);
    agent_env::set(&key, "fixture-only");
    crate::config::update_project_for_tenant(&snapshot.scope.chat.tenant, &snapshot.directory, serde_json::json!({
        "reliability": {"provider_retries":0, "provider_backoff_ms":0},
        "providers": {"default-provider": {"api": format!("http://{address}/v1"), "adapter":"openai-compatible", "env":[key.clone()], "models": {"default-model": {"status":"active", "limit":{"context":1048576, "input":900000,"output":4096}}}}}
    })).await.unwrap();
    (ModelServer { task: server, key }, requests)
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // 串行使用现有模型边界脚本队列。
async fn channel_session_history_compact_persists_summary_and_next_chat_reads_it() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    let (server, requests) = model_server(&ctx, "compressed context").await;
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("compressed context".into()),
        crate::session::llm::StreamEvent::Done {
            finish_reason: Some("stop".into()),
            usage: Default::default(),
        },
    ]);
    let prompt = lifecycle_dispatch(&ctx, &msg, &format!("/summarize {}", ids[1])).await;
    assert!(prompt.contains(&format!("/compact confirm {}", ids[1])), "{prompt}");
    let response = lifecycle_dispatch(&ctx, &msg, &format!("/compact confirm {}", ids[1])).await;
    assert!(response.contains("已压缩"), "{response}");
    assert_eq!(lifecycle_id(&ctx), id);
    let persisted = message::messages(&id, None).await.unwrap();
    assert!(persisted.iter().any(|m| m.info.id() == ids[0]));
    let summary_requests = crate::session::processor::take_test_llm_messages(&id);
    assert_eq!(summary_requests.len(), 1);
    assert!(serde_json::to_string(&summary_requests[0].messages).unwrap().contains("first turn"));
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("next answer".into()),
        crate::session::llm::StreamEvent::Done {
            finish_reason: Some("stop".into()),
            usage: Default::default(),
        },
    ]);
    let next = lifecycle_dispatch(&ctx, &msg, "next question").await;
    let captured = crate::session::processor::take_test_llm_messages(&id);
    assert!(next.contains("next answer"), "{next}; model requests={}", captured.len());
    assert_eq!(captured.len(), 1);
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    assert_eq!(Path::new(&captured[0].directory), snapshot.directory);
    assert_eq!(captured[0].tenant, snapshot.scope.chat.tenant);
    let contents = serde_json::to_string(&captured[0].messages).unwrap();
    assert!(contents.contains("compressed context"));
    assert!(contents.contains("next question"));
    assert!(!contents.contains("first turn"));
    assert!(
        ctx.conversation_histories.lock().unwrap()[&key]
            .iter()
            .any(|m| m.content.contains("next answer"))
    );
    assert!(requests.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the scripted model boundary.
async fn channel_session_history_blank_compaction_preserves_persisted_history_and_cache() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    let (server, requests) = model_server(&ctx, " ").await;
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta(" ".into()),
        crate::session::llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        },
    ]);
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    ctx.conversation_histories.lock().unwrap().insert(key.clone(), vec![ChatMessage::user("keep")]);
    let before = serde_json::to_value(message::messages(&id, None).await.unwrap()).unwrap();
    lifecycle_dispatch(&ctx, &msg, &format!("/compact {}", ids[1])).await;
    let response = lifecycle_dispatch(&ctx, &msg, &format!("/compact confirm {}", ids[1])).await;
    assert!(response.contains("失败") && response.contains("empty summary"), "{response}");
    assert_eq!(crate::session::processor::take_test_llm_messages(&id).len(), 1);
    assert!(requests.lock().unwrap().is_empty());
    assert_eq!(lifecycle_id(&ctx), id);
    assert_eq!(before, serde_json::to_value(message::messages(&id, None).await.unwrap()).unwrap());
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&key][0].content, "keep");
    server.abort();
}

#[tokio::test]
async fn channel_session_history_rejects_forged_binding_across_projects_and_tenants() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    let (_other_temp, mut other, _) = lifecycle_fixture().await;
    other.message_snapshot = None;
    let tenant = vw_api_transport::TenantScope::new("history-test", format!("user-{id}")).unwrap();
    crate::project::from_directory(&tenant, ctx.workspace_dir.as_ref()).await.unwrap();
    let mut foreign = ctx.clone();
    foreign.project_state = Arc::new(Mutex::new(Default::default()));
    foreign.project_state.lock().unwrap().tenant = tenant;
    foreign.message_snapshot = None;
    for mut candidate in [other, foreign] {
        let snapshot = resolve_channel_message_snapshot(&candidate, &msg).await.unwrap();
        // 即使内存绑定被错误填入外域 ID，也必须在底层所有权边界拒绝。
        sender_session_store().lock().unwrap().insert(snapshot.scope.clone(), id.clone());
        candidate.message_snapshot = Some(snapshot.clone());
        candidate
            .conversation_histories
            .lock()
            .unwrap()
            .insert(snapshot.scope.key(), vec![ChatMessage::user("retain")]);
        for command in ["fork", "reset", "compact"] {
            let response =
                lifecycle_dispatch(&candidate, &msg, &format!("/{command} {}", ids[1])).await;
            assert!(response.contains("失败"), "{response}");
        }
        assert_eq!(
            candidate.conversation_histories.lock().unwrap()[&snapshot.scope.key()][0].content,
            "retain"
        );
    }
    assert_eq!(lifecycle_id(&ctx), id);
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);
}

#[tokio::test]
async fn channel_session_history_switching_sessions_invalidates_confirmation() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_switching_sessions_invalidates_confirmation(ctx, msg, id, ids, dispatch_contract).await;
}

#[tokio::test]
async fn channel_session_history_part_changes_invalidate_confirmation() {
    let (_temp, ctx, msg, id, ids) = fixture().await;
    channel_runtime::command_history_contracts::channel_session_history_part_changes_invalidate_confirmation(ctx, msg, id, ids, dispatch_contract).await;
}

fn dispatch_contract<'a>(ctx: &'a ChannelRuntimeContext, msg: &'a traits::ChannelMessage, text: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + 'a>> {
    Box::pin(lifecycle_dispatch(ctx, msg, text))
}
