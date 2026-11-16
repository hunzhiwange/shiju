//! 测试 ACP 后端的会话复用、历史持久化、事件映射与取消传播。
//! 临时会话夹具同时验证持久化次数和运行时取消信号，避免只检查表面返回值。

use std::time::Duration;

use tempfile::TempDir;
use acp_server::server::{
    BackendCancelHandle, BackendEvent, BackendTermination, BackendPrompt,
    SessionBackend,
};
use vw_api_transport::id::TenantScope;

use crate::protocol::{VibeWindowBackend, Host};


#[derive(Clone, Copy)]
pub enum PromptCase { First, Followup, Native, Cancel }
#[async_trait::async_trait]
pub trait BackendPorts {
    type Host: Host + Default;
    async fn start(backend: &VibeWindowBackend<Self::Host>, id: &str, case: PromptCase) -> Result<BackendPrompt, acp_server::server::BackendError>;
}

// 为每个场景生成完整且互不重叠的租户作用域，避免会话记录串扰。
fn tenant(suffix: &str) -> TenantScope {
    TenantScope::new(format!("acp-platform-{suffix}"), format!("acp-company-{suffix}"))
        .expect("valid tenant")
}

pub async fn backend_reuses_native_session_identity_and_history<P: BackendPorts>() {
    agent_project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::<P::Host>::new(tenant("identity"));

    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let loaded = SessionBackend::load_session(&backend, &created.id, root.path().to_path_buf())
        .await
        .expect("load");

    assert_eq!(loaded.id, created.id);
    assert_eq!(loaded.cwd, created.cwd);
    assert_eq!(loaded.history, created.history);
    agent_project::instance::dispose_all().await;
}

pub async fn backend_prompt_persists_turn_once<P: BackendPorts>() {
    agent_project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::<P::Host>::new(tenant("persist"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = P::start(&backend, &created.id, PromptCase::First)
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
    let mut follow_up = P::start(&backend, &created.id, PromptCase::Followup)
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
    agent_project::instance::dispose_all().await;
}

pub async fn backend_prompt_uses_native_model_and_local_tools<P: BackendPorts>() {
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::<P::Host>::new(tenant("native-route"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = P::start(&backend, &created.id, PromptCase::Native)
    .await
    .expect("prompt");

    let terminal = tokio::time::timeout(Duration::from_secs(5), prompt.events.recv())
        .await
        .expect("terminal timeout")
        .expect("terminal event");
    assert_eq!(terminal, BackendEvent::Terminal(BackendTermination::EndTurn));
}

pub async fn backend_cancel_propagates_to_runtime_turn<P: BackendPorts>() {
    agent_project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::<P::Host>::new(tenant("cancel"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = P::start(&backend, &created.id, PromptCase::Cancel)
    .await
    .expect("prompt");

    assert!(prompt.cancel.cancel());
    assert!(!prompt.cancel.cancel());
    let terminal = tokio::time::timeout(Duration::from_secs(1), prompt.events.recv())
        .await
        .expect("terminal timeout")
        .expect("terminal event");
    assert_eq!(terminal, BackendEvent::Terminal(BackendTermination::Cancelled));
    agent_project::instance::dispose_all().await;
}
