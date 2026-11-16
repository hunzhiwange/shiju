//! 测试 ACP 后端的会话复用、历史持久化、事件映射与取消传播。
//! 临时会话夹具同时验证持久化次数和运行时取消信号，避免只检查表面返回值。

use std::time::Duration;

use tempfile::TempDir;
use acp_server::server::{
    BackendCancelHandle, BackendEvent, BackendTermination, PromptContent,
    SessionBackend,
};
use vw_api_transport::id::TenantScope;

use super::backend::{VibeWindowBackend, start_prompt_with_runtime_runner};
use crate::session::processor::StreamEvent;
use crate::session::ui_types::TokenUsage;

// 为每个场景生成完整且互不重叠的租户作用域，避免会话记录串扰。
fn tenant(suffix: &str) -> TenantScope {
    TenantScope::new(format!("acp-platform-{suffix}"), format!("acp-company-{suffix}"))
        .expect("valid tenant")
}

#[tokio::test]
async fn backend_reuses_native_session_identity_and_history() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("identity"));

    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let loaded = SessionBackend::load_session(&backend, &created.id, root.path().to_path_buf())
        .await
        .expect("load");

    assert_eq!(loaded.id, created.id);
    assert_eq!(loaded.cwd, created.cwd);
    assert_eq!(loaded.history, created.history);
    crate::project::instance::dispose_all().await;
}

#[tokio::test]
async fn backend_prompt_persists_turn_once() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("persist"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = start_prompt_with_runtime_runner(
        &backend,
        &created.id,
        vec![PromptContent::Text("hello".to_string())],
        |request, mut emit| {
            assert_eq!(
                request.reliability.provider_retries,
                crate::app::agent::config::ReliabilityConfig::default().provider_retries
            );
            emit(StreamEvent::Delta("answer".to_string()));
            emit(StreamEvent::Done(TokenUsage::default()));
        },
    )
    .await
    .expect("prompt");

    while let Some(event) = prompt.events.recv().await {
        if matches!(event, BackendEvent::Terminal(_)) {
            break;
        }
    }
    let loaded = SessionBackend::load_session(&backend, &created.id, root.path().to_path_buf())
        .await
        .expect("reload");
    assert_eq!(loaded.history.len(), 2);
    assert_eq!(loaded.history[0].text, "hello");
    assert_eq!(loaded.history[1].text, "answer");
    let mut follow_up = start_prompt_with_runtime_runner(
        &backend,
        &created.id,
        vec![PromptContent::Text("follow-up".into())],
        |request, mut emit| {
            assert_eq!(
                request.history.iter().map(|message| message.content.as_str()).collect::<Vec<_>>(),
                ["hello", "answer"]
            );
            assert_eq!(request.history[0].role, crate::session::ui_types::ChatRole::User);
            assert_eq!(request.history[1].role, crate::session::ui_types::ChatRole::Assistant);
            emit(StreamEvent::Delta("second answer".into()));
            emit(StreamEvent::Done(TokenUsage::default()));
        },
    )
    .await
    .expect("follow-up prompt");
    while let Some(event) = follow_up.events.recv().await {
        match event {
            BackendEvent::Terminal(_) => break,
            BackendEvent::Error(error) => panic!("follow-up failed: {error}"),
            _ => {}
        }
    }
    let loaded = backend.load_session(&created.id, root.path().into()).await.unwrap();
    assert_eq!(
        loaded.history.iter().map(|message| message.text.as_str()).collect::<Vec<_>>(),
        ["hello", "answer", "follow-up", "second answer"]
    );
    crate::project::instance::dispose_all().await;
}

#[tokio::test]
async fn backend_prompt_uses_native_model_and_local_tools() {
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("native-route"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = start_prompt_with_runtime_runner(
        &backend,
        &created.id,
        vec![PromptContent::Text("hello".to_string())],
        |request, mut emit| {
            assert_eq!(request.channel_name.as_deref(), Some("acp"));
            assert_eq!(request.model.as_deref(), Some("auto"));
            assert!(crate::session::processor::should_execute_structured_tool_calls_locally(
                &request.options
            ));
            emit(StreamEvent::Done(TokenUsage::default()));
        },
    )
    .await
    .expect("prompt");

    let terminal = tokio::time::timeout(Duration::from_secs(5), prompt.events.recv())
        .await
        .expect("terminal timeout")
        .expect("terminal event");
    assert_eq!(terminal, BackendEvent::Terminal(BackendTermination::EndTurn));
}

#[tokio::test]
async fn backend_cancel_propagates_to_runtime_turn() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("cancel"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = start_prompt_with_runtime_runner(
        &backend,
        &created.id,
        vec![PromptContent::Text("wait".to_string())],
        |request, mut emit| {
            let abort = request.abort_token.expect("abort token");
            while !abort.is_cancelled() {
                std::thread::yield_now();
            }
            emit(StreamEvent::Delta("ignored".to_string()));
        },
    )
    .await
    .expect("prompt");

    assert!(prompt.cancel.cancel());
    assert!(!prompt.cancel.cancel());
    let terminal = tokio::time::timeout(Duration::from_secs(1), prompt.events.recv())
        .await
        .expect("terminal timeout")
        .expect("terminal event");
    assert_eq!(terminal, BackendEvent::Terminal(BackendTermination::Cancelled));
    crate::project::instance::dispose_all().await;
}
